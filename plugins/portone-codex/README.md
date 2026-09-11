# PortOne Codex Plugin

[English](README.md) | [한국어](README.ko.md)

A Codex plugin for implementing and reviewing PortOne payment integrations.

## Features

- Generate PortOne V1 and V2 payment integration code.
- Validate existing integrations and diagnose concrete problems.
- Use official PortOne documentation and MCP examples as the source of truth.
- Use the PortOne CLI for authentication, payment inspection, and API requests.

## Installation

### Set up skills and MCP

Install the [PortOne CLI](../../README.md#installation), Node.js, and `npx`,
then configure Codex for your user account:

```bash
portone setup --agent codex --scope user
```

Setup copies the four official PortOne skills and configures the PortOne MCP
server directly. It does not install the native Codex plugin or install or
update Codex itself. Git and the Codex CLI are not required to run setup.

Update this installation explicitly with:

```bash
portone setup update --agent codex --scope user
```

Start a new Codex session after setup, inspect its MCP servers, and ask Codex
to retrieve a PortOne document. Follow any workspace trust or MCP approval
prompts from Codex. Console features may request login when used; setup does
not log in or save tokens. See the [setup guide](../../README.md#portone-setup)
for project scope, destinations, and update behavior.

### Native plugin installation

To use the native plugin, install `portone-codex` through Codex's plugin
manager using this repository's
[`portone` marketplace](../../.agents/plugins/marketplace.json).
The plugin bundles the same four skills and its own MCP configuration.
If you switch to direct setup, disable an existing plugin when it duplicates
the installed skills or server.

The native plugin's bundled `.mcp.json` uses:

```json
{
  "mcpServers": {
    "portone": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "@portone/mcp-server@latest"]
    }
  }
}
```

## Usage

Ask Codex for the integration work you need:

```text
Implement a PortOne V2 one-time payment integration.
Review the PortOne integration in this project.
Add a PortOne billing-key payment flow.
Use the PortOne CLI to inspect failed test payments.
```

## Included skills

- `payment-code-generator`: implement a new PortOne integration.
- `integration-validator`: validate an existing or newly generated integration.
- `portone-guide`: explain PortOne concepts and locate official guidance.
- `portone-cli`: authenticate and use PortOne CLI payment and API commands.

## Maintaining the bundled skills

All four directories under this plugin's `skills/` are generated copies of
the canonical directories under the repository's root `skills/`. Edit the
root sources, then synchronize the plugin copies:

```bash
cargo xtask sync-plugin-skills
cargo xtask sync-plugin-skills --check
```

Commit the source changes and all generated plugin copies together. The sync
command updates all managed skills in both plugins; the check reports missing,
changed, or stale generated files.

## License

MIT License
