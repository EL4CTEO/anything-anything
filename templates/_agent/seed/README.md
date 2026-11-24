<p align="center"><img src="docs/logo.svg" alt="{{name}}" width="112"></p>

<h1 align="center">{{name}}</h1>

<p align="center"><b>{{tagline}}</b></p>

<p align="center">
  <a href="{{repo_url}}/actions/workflows/ci.yml"><img src="{{repo_url}}/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <img src="https://img.shields.io/badge/MCP-server-555" alt="MCP server">
  <img src="https://img.shields.io/badge/Claude%20Code-plugin-d97757" alt="Claude Code plugin">
  <a href="https://github.com/{{home}}"><img src="https://img.shields.io/badge/made%20with-anything--anything-{{color_hex}}" alt="made with anything-anything"></a>
</p>

{{pitch}}

## Install

```sh
{{install}}
```

## Use

```sh
{{usage}}
```

{{commands_table_md}}

Every command takes `--json`.

## Agents

- **Claude Code plugin**: `/plugin marketplace add {{repo}}`, then `/plugin install {{name}}@{{name}}`
- **MCP server**, built in: `claude mcp add {{name}} -- {{name}} mcp`. Tools: {{mcp_tools_md}}
- **Skill**: [`skills/{{name}}`](skills/{{name}}/SKILL.md)

## License

MIT
