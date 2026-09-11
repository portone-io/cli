# portone setup update

English | [한국어](ko/portone_setup_update.md)

Update recorded PortOne skills and MCP settings

```
portone setup update [OPTIONS]
```

## Options

| Option | Description |
| --- | --- |
| `--dry-run` | Preview updates without writing files |
| `--agent <AGENT>` | Agents to configure (comma-separated or repeated) [possible values: claude-code, codex, cursor, gemini-cli, github-copilot, vscode-copilot, opencode] |
| `--scope <SCOPE>` | Installation scope (project \| user) [possible values: project, user] |

## See also

- [portone setup](portone_setup.md)
