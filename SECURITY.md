# Security Policy

llmignore is a tool whose whole job is to keep sensitive files away from AI assistants, so
we take its own security seriously.

## Reporting a vulnerability

Please report security issues privately via
[GitHub Security Advisories](https://github.com/horiastanxd/llmignore/security/advisories/new)
rather than a public issue. You will get an acknowledgement within a few days.

## Scope

Particularly interested in reports where:

- A file that should be ignored (a secret, a key) is classified as **included**.
- `llmignore scan` fails to flag a known secret pattern (a false negative).
- The npm installer or `install.sh` could be tricked into running or installing
  untrusted code.

## Good to know

- The binary makes **no network calls**. It only reads the local filesystem.
- The npm package downloads a prebuilt binary from this repository's GitHub Releases over
  HTTPS; checksums for every artifact are published alongside the binaries.
