<p align="center"><img src="docs/logo.svg" alt="secrets-anything" width="112"></p>

<h1 align="center">secrets-anything</h1>

<p align="center"><b>No secret leaves your machine.</b></p>

<p align="center">
  <a href="https://github.com/EL4CTEO/secrets-anything/actions/workflows/ci.yml"><img src="https://github.com/EL4CTEO/secrets-anything/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <img src="https://img.shields.io/badge/MCP-server-555" alt="MCP server">
  <img src="https://img.shields.io/badge/Claude%20Code-plugin-d97757" alt="Claude Code plugin">
  <a href="https://github.com/EL4CTEO/anything-anything"><img src="https://img.shields.io/badge/made%20with-anything--anything-dc2626" alt="made with anything-anything"></a>
</p>

Find leaked API keys, tokens, passwords and private keys in any codebase, in any language, before you push. Fast, offline, no false alarms on test fixtures.

## Install

```sh
npm i -g @el4cteo/secrets-anything   # or: cargo install secrets-anything
```

## Use

```console
$ secrets-anything scan .
config.py:2:12  aws-access-key  AKIA****
deploy/id_ed25519:1:1  private-key  ----****
settings.yaml:2:13  password-assignment  Tr0u****
3 secrets found
```

Exits 1 when it finds something, so it drops into a pre-commit hook or CI as is:

```sh
git diff --cached | secrets-anything check   # just what you're about to commit
```

| command | what it does |
|---|---|
| `secrets-anything scan [path] [--all]` | Find leaked secrets in a folder or file |
| `secrets-anything check [--name <name>]` | Find leaked secrets in text from stdin |
| `secrets-anything rules` | List the built-in detection rules |

- **15 built-in rules**: private keys, AWS, GitHub, GitLab, Slack, Stripe, Google, OpenAI, Anthropic, npm, JWTs, passwords in URLs, and strong literals assigned to `password`/`secret`/`token`/`api_key`-style keys.
- **Fast**: parallel walk, one regex set per line. Honors `.gitignore`, `.ignore` and `.git/info/exclude`; skips `node_modules`, `target`, `dist`, lock files, minified, binary and 1 MiB+ files.
- **No false alarms**: fixture folders (`fixtures`, `testdata`, `__mocks__`, ...), `.example`/`.sample`/`.template` files and placeholder values (`your-key`, `${VAR}`, `xxxx`, `changeme`, ...) are skipped; `--all` shows them. Silence a line with a `secrets-anything: allow` comment.
- **Never prints a secret**: only its first 4 characters, in text, JSON and MCP.

Every command takes `--json`.

## Agents

- **Claude Code plugin**: `/plugin marketplace add EL4CTEO/secrets-anything`, then `/plugin install secrets-anything@secrets-anything`
- **MCP server**, built in: `claude mcp add secrets-anything -- secrets-anything mcp`. Tools: `secrets_scan`, `secrets_check`, `secrets_rules`
- **Skill**: [`skills/secrets-anything`](skills/secrets-anything/SKILL.md)

## License

MIT
