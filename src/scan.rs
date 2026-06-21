//! Detect sensitive files (secrets) that are NOT yet ignored - the killer feature.

use crate::defaults::SENSITIVE_PATTERNS;
use crate::{scan, ScanOptions};
use ignore::gitignore::GitignoreBuilder;
use std::path::{Path, PathBuf};

/// A sensitive file found in the repo that an AI tool could currently read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Path relative to the scanned root.
    pub path: PathBuf,
    /// Human-readable reason this file is risky.
    pub reason: String,
}

/// Find sensitive files that are currently NOT ignored (i.e. exposed to AI).
pub fn find_exposed_secrets(root: &Path) -> anyhow::Result<Vec<Finding>> {
    let mut matcher = GitignoreBuilder::new(root);
    for (glob, _) in SENSITIVE_PATTERNS {
        matcher.add_line(None, glob)?;
    }
    let matcher = matcher.build()?;

    let opts = ScanOptions {
        compute_ignored: false,
        ..Default::default()
    };
    let result = scan(root, &opts)?;

    let mut findings = Vec::new();
    for path in &result.included {
        let m = matcher.matched(path, false);
        if m.is_ignore() {
            let reason = reason_for(path);
            findings.push(Finding {
                path: crate::relativize(path, root),
                reason,
            });
        }
    }
    findings.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(findings)
}

fn reason_for(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    for (glob, reason) in SENSITIVE_PATTERNS {
        if glob_matches_name(glob, &name) {
            return reason.to_string();
        }
    }
    "Potentially sensitive file".to_string()
}

fn glob_matches_name(glob: &str, name: &str) -> bool {
    if let Some(suffix) = glob.strip_prefix('*') {
        name.ends_with(suffix)
    } else if let Some(prefix) = glob.strip_suffix('*') {
        name.starts_with(prefix)
    } else {
        glob == name
    }
}
