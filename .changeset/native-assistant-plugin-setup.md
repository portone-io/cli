---
"@portone/cli": minor
---

Install four official PortOne skills and MCP configuration for Claude Code, Codex, Cursor, Gemini CLI, GitHub Copilot CLI, VS Code Copilot, and OpenCode. Select multiple agents with `--agent` and choose project or user scope with `--scope`.

Add `portone setup update` to refresh recorded installations from the latest official release independently of the CLI version, with agent/scope filters and `--dry-run`. Preserve unrelated settings and comments, repair missing or modified managed files, and roll back a file replacement if its installation record cannot be saved.

Remove the deprecated `--assistant` and `--allow-dirty` options. Non-interactive setup requires explicit `--agent` and `--scope` options. Existing native plugins are not removed automatically. Keep the canonical skills synchronized into the published plugin bundles, including Claude agent access to the bundled MCP tools.
