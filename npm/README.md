# llmignore

Like `.gitignore`, but it tells AI tools (Claude Code, Cursor, Copilot, ...) what **not** to read.

```bash
npx llmignore-cli init     # write a .llmignore with strong defaults
npx llmignore-cli scan     # any secrets currently exposed to AI? (exit 1 if yes)
npx llmignore-cli sync     # mirror it into .cursorignore, .aiexclude, ...
```

This package (`llmignore-cli`) installs a small native binary (built in Rust) for your
platform on first use. The installed command is `llmignore`.

Full docs: https://github.com/horiastanxd/llmignore
