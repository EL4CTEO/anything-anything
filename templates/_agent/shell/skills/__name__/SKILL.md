---
name: {{name}}
description: {{description_json}}
---

# {{name}}

{{pitch}}

## Commands

{{skill_commands_md}}

Add `--json` to any command for machine-readable output. The same commands are MCP tools ({{mcp_tools_md}}), served by `{{name}} mcp`.

## Use it well

- Start with `{{usage}}`.
- Use `--json` (or the MCP tools) when you'll read the result yourself; show people the plain text.
- A non-zero exit means it failed or found what it was asked to fail on; the reason is on stderr.
