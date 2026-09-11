# PortOne Integration Plugin

[English](README.md) | [한국어](README.ko.md)

Implement and review PortOne V1 and V2 integrations in Claude Code.
Supports one-time payments, billing-key payments, key-in payments, and identity
verification using official documentation and MCP tools.

## Installation

Install `portone-integration` through Claude Code's plugin manager using this
repository's [`portone` marketplace](../../.claude-plugin/marketplace.json).
Node.js and `npx` are required to run the bundled MCP server.

The plugin includes the `/portone-integration:start` command, code generation
and review agents, and the `portone-cli` and `portone-guide` skills.

## Usage

Start an integration interactively or specify a version and payment type:

```text
/portone-integration:start
/portone-integration:start v2 checkout
```

Versions: `v1`, `v2`. Types: `checkout`, `billing`, `keyin`, `identity`.
You can also ask Claude directly:

```text
Review the PortOne integration in src/payment/.
Use the PortOne CLI to inspect failed test payments.
```

## CLI setup

Install the [PortOne CLI](../../README.md#installation), then configure skills
and MCP settings:

```sh
portone setup --agent claude-code --scope user
portone setup update --agent claude-code --scope user
```

CLI setup includes four skills for natural language requests. The `/start`
command and specialized agents require the native plugin.
Restart Claude Code after setup and check the server with `/mcp`.
If you switch from the plugin to CLI setup, disable the plugin to avoid duplicates.
See the [setup guide](../../docs/setup.md) for scopes and installation paths.

## Contributing

Edit `portone-cli` and `portone-guide` under the repository's `skills/` directory,
then [synchronize the plugin copies](../../docs/development.md).
The plugin's `commands/` and `agents/` are maintained separately.

## License

MIT
