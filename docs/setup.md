# Agent setup

English | [한국어](setup.ko.md)

`portone setup` installs four official skills (`portone-cli`, `portone-guide`,
`payment-code-generator`, `integration-validator`) and configures the PortOne
MCP server. Node.js and `npx` are required; the server runs
`npx -y @portone/mcp-server@latest`.

## Install

Run interactively or specify agents and scope:

```sh
portone setup
portone setup --agent codex --scope project
portone setup --agent claude-code,cursor --scope user
```

`--agent` accepts comma-separated or repeated values. `--scope` is `project`
or `user`; both options are required for noninteractive setup.
The legacy `--assistant claude|codex|both` option uses user scope.

## Destinations

Project scope writes to these paths:

| Agent ID | Skills | MCP configuration |
| --- | --- | --- |
| `claude-code` | `.claude/skills` | `.mcp.json` |
| `codex` | `.agents/skills` | `.codex/config.toml` |
| `cursor` | `.agents/skills` | `.cursor/mcp.json` |
| `gemini-cli` | `.agents/skills` | `.gemini/settings.json` |
| `github-copilot` | `.agents/skills` | `.mcp.json` |
| `vscode-copilot` | `.agents/skills` | `.vscode/mcp.json` |
| `opencode` | `.agents/skills` | `opencode.jsonc` if present, otherwise `opencode.json` |

User scope uses each agent's standard configuration directory and honors
`CODEX_HOME`, `CLAUDE_CONFIG_DIR`, `GEMINI_CLI_HOME`, `COPILOT_HOME`, and
`XDG_CONFIG_HOME` where applicable. OpenCode installs under
`${XDG_CONFIG_HOME:-~/.config}/opencode` regardless of `OPENCODE_CONFIG` and
`OPENCODE_CONFIG_DIR`. VS Code targets the stable/default profile.

Agents sharing a destination use one copy. Setup replaces the managed skills
and `portone` MCP entry while preserving unrelated settings and supported
JSONC/TOML comments.

## Update

```sh
portone setup update
portone setup update --agent codex --scope user
portone setup update --dry-run
```

`--dry-run` previews updates without requiring Node.js or `npx`.

Updates use installation receipts in `.portone/setup.json` for the current
project and `setup.json` in the PortOne configuration directory for user scope.
`PORTONE_CONFIG_DIR` overrides the latter directory. Without filters, updates
check both scopes. Updates run explicitly and independently of CLI updates.

Setup downloads skills and MCP files from one commit in the latest official
GitHub release. It uses the default branch only when no latest release exists;
a release missing the canonical skills causes setup to fail.
GitHub requests optionally use `GH_TOKEN`, then `GITHUB_TOKEN`.

## After setup

Restart the agent, check its MCP servers, and ask it to retrieve a PortOne
document. Follow the agent's workspace trust or MCP approval prompts.
Setup configures files; Console features may request login when used.

Native plugins are also available for [Codex](../plugins/portone-codex/README.md)
and [Claude Code](../plugins/portone-integration/README.md). When switching to
CLI setup, disable any existing plugin that duplicates the skills or MCP server.
