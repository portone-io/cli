# portone setup

Install PortOne skills and MCP settings for AI coding agents

```
portone setup [OPTIONS] [COMMAND]
```

## Commands

| Command | Description |
| --- | --- |
| [portone setup update](portone_setup_update.md) | Update recorded PortOne skills and MCP settings |

## Options

| Option | Description |
| --- | --- |
| `--agent <AGENT>` | Agents to configure (comma-separated or repeated) [possible values: claude-code, codex, cursor, gemini-cli, github-copilot, vscode-copilot, opencode] |
| `--scope <SCOPE>` | Installation scope (project \| user) [possible values: project, user] |
| `--assistant <ASSISTANT>` | Legacy selection (claude \| codex \| both); defaults to user scope |

## See also

- [portone](portone.md)
