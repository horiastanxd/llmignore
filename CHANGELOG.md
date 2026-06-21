# Changelog

All notable changes to this project are documented here. Format based on
[Keep a Changelog](https://keepachangelog.com/), versioning per [SemVer](https://semver.org/).

## [0.1.0] - 2026-06-21

Initial release.

### Added
- `.llmignore` file convention - same syntax as `.gitignore`, tells AI tools what not to read.
- Built on ripgrep's `ignore` engine: parallel-capable walk, gitignore semantics, `.gitignore` layering.
- Comprehensive built-in default ruleset (secrets, dependencies, build output, caches, lockfiles, binaries, media, logs), used as a fallback when no `.llmignore` exists.
- `init` - write a `.llmignore` with the default ruleset.
- `scan` - report secret files currently exposed to AI tools; exits 1 if any (CI-friendly).
- `sync` - mirror `.llmignore` into `.cursorignore`, `.codeiumignore`, `.aiexclude`, `.geminiignore`, `.aiderignore`.
- `list` - list included (or `--ignored`) files; `--json`, `-0/--null`, `--absolute`.
- `check <file>` - exit 0 if an AI would read the file, 1 if ignored.
- `stats` - included vs ignored counts and an estimated token total.
- Global flags `-C/--dir`, `--no-gitignore`, `--no-defaults`; `--json` on scan/list/stats.
- Distribution: crates.io (`llmignore-cli`), npm (`npx llmignore-cli`, prebuilt binaries), `curl | sh` installer. Package name is `llmignore-cli`; the command is `llmignore`.

[0.1.0]: https://github.com/horiastanxd/llmignore/releases/tag/v0.1.0
