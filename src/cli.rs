use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use llmignore::scan::find_exposed_secrets;
use llmignore::sync::{sync, SyncStatus};
use llmignore::tokens::estimate_file_tokens;
use llmignore::{relativize, scan, ScanOptions, DEFAULT_LLMIGNORE};

/// Exit code returned when the action found a problem (exposed secret, ignored
/// file, missing target) - chosen so scripts can branch on it.
const EXIT_PROBLEM: i32 = 1;

#[derive(Parser, Debug)]
#[command(
    name = "llmignore",
    version,
    about = "Like .gitignore, but tells AI tools what NOT to read.",
    long_about = "llmignore scans a project the way an AI assistant (Claude Code, Cursor, \
Copilot, ...) would, honoring a .llmignore file (same syntax as .gitignore) so secrets, \
dependencies and build output never reach the model.",
    propagate_version = true
)]
struct Cli {
    /// Run as if started in DIR instead of the current directory.
    #[arg(short = 'C', long = "dir", global = true, value_name = "DIR")]
    dir: Option<PathBuf>,

    /// Ignore .gitignore / git excludes (use only .llmignore).
    #[arg(long = "no-gitignore", global = true)]
    no_gitignore: bool,

    /// Do not fall back to the built-in default ruleset when no .llmignore exists.
    #[arg(long = "no-defaults", global = true)]
    no_defaults: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Write a .llmignore with sensible defaults.
    Init {
        /// Overwrite an existing .llmignore.
        #[arg(short, long)]
        force: bool,
    },
    /// List files an AI tool would read (default), or the ignored ones.
    List {
        /// Show the ignored files instead of the included ones.
        #[arg(long)]
        ignored: bool,
        /// Output JSON.
        #[arg(long)]
        json: bool,
        /// Separate paths with NUL instead of newline (for `xargs -0`).
        #[arg(short = '0', long = "null")]
        null: bool,
        /// Print absolute paths.
        #[arg(long)]
        absolute: bool,
    },
    /// Check whether a single file would be read by an AI tool.
    Check {
        /// File to check.
        file: String,
        /// Print nothing; rely on the exit code only.
        #[arg(short, long)]
        quiet: bool,
    },
    /// Find secret files currently exposed to AI tools (exit 1 if any).
    Scan {
        /// Output JSON.
        #[arg(long)]
        json: bool,
    },
    /// Mirror .llmignore into the ignore files AI tools read today (.cursorignore, ...).
    Sync {
        /// Overwrite tool ignore files even if hand-written.
        #[arg(short, long)]
        force: bool,
    },
    /// Show a summary: included vs ignored files and estimated tokens.
    Stats {
        /// Output JSON.
        #[arg(long)]
        json: bool,
    },
    /// Print a shell completion script (bash, zsh, fish, powershell, elvish).
    Completions {
        /// Target shell.
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Print a man page (roff) to stdout.
    Man,
}

/// Parse arguments and run. Returns the process exit code.
pub fn run() -> i32 {
    let cli = Cli::parse();
    match dispatch(&cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{} {:#}", paint("error:", Color::Red), e);
            2
        }
    }
}

fn dispatch(cli: &Cli) -> Result<i32> {
    let root = cli.dir.clone().unwrap_or_else(|| PathBuf::from("."));
    let opts = ScanOptions {
        use_gitignore: !cli.no_gitignore,
        use_defaults: !cli.no_defaults,
        compute_ignored: false,
        follow_links: false,
    };

    match &cli.command {
        Some(Command::Init { force }) => cmd_init(&root, *force),
        Some(Command::List {
            ignored,
            json,
            null,
            absolute,
        }) => cmd_list(&root, &opts, *ignored, *json, *null, *absolute),
        Some(Command::Check { file, quiet }) => cmd_check(&root, &opts, file, *quiet),
        Some(Command::Scan { json }) => cmd_scan(&root, *json),
        Some(Command::Sync { force }) => cmd_sync(&root, *force),
        Some(Command::Stats { json }) => cmd_stats(&root, &opts, *json),
        Some(Command::Completions { shell }) => cmd_completions(*shell),
        Some(Command::Man) => cmd_man(),
        None => cmd_stats(&root, &opts, false),
    }
}

fn cmd_completions(shell: clap_complete::Shell) -> Result<i32> {
    let mut cmd = <Cli as clap::CommandFactory>::command();
    clap_complete::generate(shell, &mut cmd, "llmignore", &mut std::io::stdout());
    Ok(0)
}

fn cmd_man() -> Result<i32> {
    let cmd = <Cli as clap::CommandFactory>::command();
    clap_mangen::Man::new(cmd).render(&mut std::io::stdout())?;
    Ok(0)
}

fn cmd_init(root: &Path, force: bool) -> Result<i32> {
    let target = root.join(".llmignore");
    if target.exists() && !force {
        bail!(
            ".llmignore already exists at {} (use --force to overwrite)",
            target.display()
        );
    }
    std::fs::write(&target, DEFAULT_LLMIGNORE)
        .with_context(|| format!("writing {}", target.display()))?;
    let n = DEFAULT_LLMIGNORE.lines().filter(|l| is_rule(l)).count();
    println!(
        "{} {} ({} rules). AI tools will now skip secrets, deps and build output.",
        paint("Created", Color::Green),
        target.display(),
        n
    );
    Ok(0)
}

fn cmd_list(
    root: &Path,
    opts: &ScanOptions,
    ignored: bool,
    json: bool,
    null: bool,
    absolute: bool,
) -> Result<i32> {
    let opts = ScanOptions {
        compute_ignored: ignored,
        ..opts.clone()
    };
    let result = scan(root, &opts).context("scanning")?;
    let files = if ignored {
        &result.ignored
    } else {
        &result.included
    };

    if json {
        let key = if ignored { "ignored" } else { "included" };
        let list: Vec<String> = files
            .iter()
            .map(|p| display_path(p, root, absolute))
            .collect();
        let out = serde_json::json!({
            "root": root.display().to_string(),
            "count": list.len(),
            key: list,
            "used_defaults": result.used_defaults,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(0);
    }

    let sep = if null { b'\0' } else { b'\n' };
    let stdout = std::io::stdout();
    let mut w = stdout.lock();
    for p in files {
        w.write_all(display_path(p, root, absolute).as_bytes())?;
        w.write_all(&[sep])?;
    }
    Ok(0)
}

fn cmd_check(root: &Path, opts: &ScanOptions, file: &str, quiet: bool) -> Result<i32> {
    let opts = ScanOptions {
        compute_ignored: true,
        ..opts.clone()
    };
    let result = scan(root, &opts).context("scanning")?;
    let needle = normalize(file);

    let included = result
        .included
        .iter()
        .any(|p| relativize(p, root).to_string_lossy() == needle);
    if included {
        if !quiet {
            println!(
                "{} {} will be read by AI tools",
                paint("✓", Color::Green),
                file
            );
        }
        return Ok(0);
    }

    let is_ignored = result
        .ignored
        .iter()
        .any(|p| relativize(p, root).to_string_lossy() == needle);
    if is_ignored {
        if !quiet {
            println!(
                "{} {} is ignored (AI tools will skip it)",
                paint("✗", Color::Yellow),
                file
            );
        }
        return Ok(EXIT_PROBLEM);
    }

    bail!("{} not found under {}", file, root.display());
}

fn cmd_scan(root: &Path, json: bool) -> Result<i32> {
    let findings = find_exposed_secrets(root).context("scanning for secrets")?;

    if json {
        let items: Vec<_> = findings
            .iter()
            .map(
                |f| serde_json::json!({ "path": f.path.display().to_string(), "reason": f.reason }),
            )
            .collect();
        let out = serde_json::json!({ "exposed": items, "count": items.len() });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(if findings.is_empty() { 0 } else { EXIT_PROBLEM });
    }

    if findings.is_empty() {
        println!(
            "{} No exposed secrets. AI tools cannot read sensitive files here.",
            paint("✓", Color::Green)
        );
        return Ok(0);
    }

    println!(
        "{} {} exposed secret{} reachable by AI tools:\n",
        paint("⚠", Color::Red),
        findings.len(),
        if findings.len() == 1 { "" } else { "s" }
    );
    for f in &findings {
        println!(
            "  {}  {}",
            paint(&f.path.display().to_string(), Color::Red),
            dim(&f.reason)
        );
    }
    println!("\nAdd them to .llmignore (or run `llmignore init`).");
    Ok(EXIT_PROBLEM)
}

fn cmd_sync(root: &Path, force: bool) -> Result<i32> {
    let outcomes = sync(root, force)?;
    let written: Vec<_> = outcomes
        .iter()
        .filter(|o| o.status == SyncStatus::Written)
        .collect();
    let skipped: Vec<_> = outcomes
        .iter()
        .filter(|o| o.status == SyncStatus::SkippedExists)
        .collect();

    for o in &written {
        println!(
            "{} {}  {}",
            paint("synced", Color::Green),
            o.filename,
            dim(o.tool)
        );
    }
    for o in &skipped {
        println!(
            "{} {}  {} (hand-written, use --force)",
            paint("skipped", Color::Yellow),
            o.filename,
            dim(o.tool)
        );
    }
    println!(
        "\n{} tool file{} written from .llmignore.",
        written.len(),
        if written.len() == 1 { "" } else { "s" }
    );
    Ok(0)
}

fn cmd_stats(root: &Path, opts: &ScanOptions, json: bool) -> Result<i32> {
    let opts = ScanOptions {
        compute_ignored: true,
        ..opts.clone()
    };
    let result = scan(root, &opts).context("scanning")?;
    let included_tokens: u64 = result
        .included
        .iter()
        .map(|p| estimate_file_tokens(p))
        .sum();
    let findings = find_exposed_secrets(root).unwrap_or_default();

    let rules = match (
        opts.use_defaults && result.used_defaults,
        opts.use_gitignore,
    ) {
        (true, true) => "built-in defaults + .gitignore",
        (true, false) => "built-in defaults",
        (false, true) => ".llmignore + .gitignore",
        (false, false) => ".llmignore",
    };

    if json {
        let out = serde_json::json!({
            "root": root.display().to_string(),
            "included_files": result.included.len(),
            "ignored_files": result.ignored.len(),
            "included_tokens_estimate": included_tokens,
            "exposed_secrets": findings.len(),
            "rules": rules,
            "used_defaults": result.used_defaults,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(if findings.is_empty() { 0 } else { EXIT_PROBLEM });
    }

    println!(
        "{}  {}\n",
        paint("llmignore", Color::Cyan),
        dim(&root.display().to_string())
    );
    println!(
        "  {}  {:>7} files   {} tokens",
        pad("Included", 9),
        fmt_count(result.included.len()),
        paint(&format!("~{}", human_tokens(included_tokens)), Color::Cyan)
    );
    println!(
        "  {}  {:>7} files",
        pad("Ignored", 9),
        fmt_count(result.ignored.len())
    );
    println!("  {}  {}", pad("Rules", 9), dim(rules));

    if !findings.is_empty() {
        println!(
            "\n  {} {} exposed secret{} (run `llmignore scan`)",
            paint("⚠", Color::Red),
            findings.len(),
            if findings.len() == 1 { "" } else { "s" }
        );
    }
    Ok(if findings.is_empty() { 0 } else { EXIT_PROBLEM })
}

// ── helpers ──

fn is_rule(line: &str) -> bool {
    let t = line.trim();
    !t.is_empty() && !t.starts_with('#')
}

fn normalize(file: &str) -> String {
    file.strip_prefix("./").unwrap_or(file).to_string()
}

fn display_path(p: &Path, root: &Path, absolute: bool) -> String {
    if absolute {
        std::fs::canonicalize(p)
            .unwrap_or_else(|_| p.to_path_buf())
            .display()
            .to_string()
    } else {
        relativize(p, root).display().to_string()
    }
}

fn fmt_count(n: usize) -> String {
    let s = n.to_string();
    let bytes: Vec<char> = s.chars().rev().collect();
    let mut out = String::new();
    for (i, c) in bytes.iter().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(*c);
    }
    out.chars().rev().collect()
}

fn human_tokens(t: u64) -> String {
    if t >= 1_000_000 {
        format!("{:.1}M", t as f64 / 1_000_000.0)
    } else if t >= 1_000 {
        format!("{:.1}k", t as f64 / 1_000.0)
    } else {
        t.to_string()
    }
}

fn pad(s: &str, width: usize) -> String {
    format!("{s:<width$}")
}

// ── tiny ANSI coloring (honors NO_COLOR and non-tty) ──

#[derive(Clone, Copy)]
enum Color {
    Red,
    Green,
    Yellow,
    Cyan,
}

fn color_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}

fn paint(s: &str, c: Color) -> String {
    if !color_enabled() {
        return s.to_string();
    }
    let code = match c {
        Color::Red => "31",
        Color::Green => "32",
        Color::Yellow => "33",
        Color::Cyan => "36",
    };
    format!("\x1b[{code}m{s}\x1b[0m")
}

fn dim(s: &str) -> String {
    if !color_enabled() {
        return s.to_string();
    }
    format!("\x1b[2m{s}\x1b[0m")
}
