//! llmignore - fast native engine for `.llmignore` files.
//!
//! `.llmignore` uses the exact same syntax as `.gitignore`, but its purpose is to
//! tell AI tools (Claude Code, Cursor, Copilot, ...) which files they must NOT read:
//! secrets, dependencies, build output, binaries, and other noise.

use anyhow::Context;
use ignore::WalkBuilder;
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

pub mod defaults;
pub mod scan;
pub mod sync;
pub mod tokens;

pub use defaults::DEFAULT_LLMIGNORE;

/// Options controlling how a repository tree is classified.
#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// Also honor `.gitignore`, `.git/info/exclude`, and the global gitignore.
    pub use_gitignore: bool,
    /// When no `.llmignore` exists at the root, fall back to the embedded defaults.
    pub use_defaults: bool,
    /// Compute the `ignored` set too (requires a full enumeration walk).
    pub compute_ignored: bool,
    /// Follow symbolic links while walking.
    pub follow_links: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            use_gitignore: true,
            use_defaults: true,
            compute_ignored: false,
            follow_links: false,
        }
    }
}

/// Result of classifying a directory tree.
#[derive(Debug, Default)]
pub struct ScanResult {
    /// Files an AI tool SHOULD read (not ignored).
    pub included: Vec<PathBuf>,
    /// Files that ARE ignored (only populated when `compute_ignored` is set).
    pub ignored: Vec<PathBuf>,
    /// True when the embedded default ruleset was applied (no root `.llmignore`).
    pub used_defaults: bool,
}

/// Classify every file under `root` into included / ignored sets.
pub fn scan(root: &Path, opts: &ScanOptions) -> anyhow::Result<ScanResult> {
    let root_has_llmignore = root.join(".llmignore").exists();
    let apply_defaults = opts.use_defaults && !root_has_llmignore;

    // Keep the temp defaults file alive for the duration of the walk.
    let _defaults_tmp = if apply_defaults {
        Some(write_defaults_tempfile().context("writing default ruleset")?)
    } else {
        None
    };

    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false) // we decide what to hide via patterns, not by leading dot
        .parents(false) // stay scoped to `root`
        .ignore(false) // don't read generic `.ignore`/`.rgignore`
        .git_ignore(opts.use_gitignore)
        .git_global(opts.use_gitignore)
        .git_exclude(opts.use_gitignore)
        .require_git(false) // honor .gitignore even outside a git repo
        .follow_links(opts.follow_links)
        .add_custom_ignore_filename(".llmignore");

    if let Some(ref tmp) = _defaults_tmp {
        if let Some(err) = builder.add_ignore(tmp.path()) {
            return Err(anyhow::anyhow!("loading default ruleset: {err}"));
        }
    }

    let mut included = Vec::new();
    for dent in builder.build() {
        let dent = dent.context("walking directory tree")?;
        if dent.file_type().is_some_and(|ft| ft.is_file()) {
            included.push(dent.into_path());
        }
    }
    included.sort();

    let mut ignored = Vec::new();
    if opts.compute_ignored {
        let included_set: HashSet<&PathBuf> = included.iter().collect();
        for dent in baseline_walk(root, opts.follow_links) {
            let dent = dent.context("enumerating files")?;
            if !dent.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }
            let path = dent.path();
            if path.components().any(|c| c.as_os_str() == ".git") {
                continue; // .git internals are pure noise, never "ignored content"
            }
            let path = path.to_path_buf();
            if !included_set.contains(&path) {
                ignored.push(path);
            }
        }
        ignored.sort();
    }

    Ok(ScanResult {
        included,
        ignored,
        used_defaults: apply_defaults,
    })
}

/// Enumerate every file under `root` with no ignore rules applied.
fn baseline_walk(root: &Path, follow_links: bool) -> ignore::Walk {
    let mut builder = WalkBuilder::new(root);
    builder
        .standard_filters(false)
        .hidden(false)
        .parents(false)
        .follow_links(follow_links);
    builder.build()
}

fn write_defaults_tempfile() -> anyhow::Result<tempfile::NamedTempFile> {
    let mut tmp = tempfile::Builder::new()
        .prefix("llmignore-defaults-")
        .tempfile()?;
    tmp.write_all(defaults::DEFAULT_LLMIGNORE.as_bytes())?;
    tmp.flush()?;
    Ok(tmp)
}

/// Make `path` relative to `root` for display, falling back to the original path.
pub fn relativize(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|_| path.to_path_buf())
}
