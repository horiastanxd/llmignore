# Contributing

Thanks for considering a contribution. llmignore is a small, focused Rust CLI - easy to
hack on.

## Setup

```bash
git clone https://github.com/horiastanxd/llmignore
cd llmignore
cargo test        # runs the full suite
cargo run -- stats
```

## Before opening a PR

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all
```

CI runs exactly these three.

## Especially welcome

- **New `sync` targets.** If an AI tool reads a gitignore-style file, add it to
  `TARGETS` in `src/sync.rs` (filename + tool name) and a test.
- **Default ruleset additions.** Secrets, dependency dirs, or build output that should be
  ignored out of the box go in `src/defaults.rs`. Add a matching `SENSITIVE_PATTERNS` entry
  if it is a secret.
- **Secret detection patterns** for `llmignore scan`.

## Conventions

- Tests first - every behavior change ships with a test (see `tests/`).
- Keep the binary dependency-light and the default output friendly.
- The file format is gitignore syntax. No custom syntax, ever.
