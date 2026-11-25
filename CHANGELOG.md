# Changelog

## 0.2.0

- `verify --example <text>` (`-e`, repeatable): re-run only the examples whose names match. Also on the MCP tool (`anything_verify` `examples`). Partial runs aren't saved as the last verify.
- Fix: http examples ignored their `timeout` (always 30s); now they use it (default 60s, like every example).
- Fix: shell examples could match `stdout` text spanning the end of stdout and the start of stderr.

## 0.1.0

- `new`: an idea becomes a project. Your agent designs `anything.toml`, anything-anything generates the project, your agent builds it until every gate passes.
- Five stacks: `rust-cli`, `python-cli`, `node-cli` (TypeScript on Node 24+, no build step), `web` (Vite + TypeScript: apps and games), `any` (any language, its own steps).
- `verify`: spec, generated files, stubs, the stack's format/build/lint/test, MCP and every example (CLI, shell or HTTP), on Windows, macOS and Linux.
- Every CLI it makes ships an MCP server (2026-07-28 and older clients), a Claude Code plugin, a skill, CI and a release workflow.
- `hub`: every -anything tool on GitHub, and how to install it. `publish`: GitHub repo, topics and release tag.
- Its own MCP server, Claude Code plugin and skill.
