---
name: secrets-anything
description: "Find leaked API keys, tokens, passwords and private keys in any codebase, in any language, before you push. Fast, offline, no false alarms on test fixtures. Use when the user is about to commit or push, asks whether a repo, file or diff contains leaked credentials, or wants text checked for API keys, tokens, passwords or private keys."
---

# secrets-anything

Find leaked API keys, tokens, passwords and private keys in any codebase, in any language, before you push. Fast, offline, no false alarms on test fixtures.

## Commands

- `secrets-anything scan [path] [--all]`: Find leaked secrets in a folder or file
  - `[path]`: Folder or file to scan
  - `[--all]`: Also report matches in test fixtures and placeholder values
- `secrets-anything check [--name <name>]`: Find leaked secrets in text from stdin
  - `[--name <name>]`: File name to report the text as; drives the fixture and config-file rules
- `secrets-anything rules`: List the built-in detection rules

Add `--json` to any command for machine-readable output. The same commands are MCP tools (`secrets_scan`, `secrets_check`, `secrets_rules`), served by `secrets-anything mcp`.

## Use it well

- Start with `secrets-anything scan .`.
- Use `--json` (or the MCP tools) when you'll read the result yourself; show people the plain text.
- A non-zero exit means it failed or found what it was asked to fail on; the reason is on stderr.
