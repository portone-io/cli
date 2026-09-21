---
"@portone/cli": minor
---

Update `portone setup` to install four official PortOne skills and MCP configuration for Claude Code, Codex, Cursor, Gemini CLI, GitHub Copilot CLI, VS Code Copilot, and OpenCode. Support multiple agents with `--agent` and project or user scope with `--scope`.

Add `portone setup update` to refresh recorded installations from the latest official release independently of the CLI version, with agent/scope filters, `--dry-run`, and repair of missing or modified managed files.

Remove `--assistant` and `--allow-dirty`. Require explicit `--agent` and `--scope` options for non-interactive setup.

Enable bundled MCP tools for Claude agents.
