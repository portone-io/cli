# PortOne Codex Plugin

[English](README.md) | [한국어](README.ko.md)

Implement and review PortOne payment integrations in Codex using official
documentation, MCP tools, and the PortOne CLI.

## Installation

Install `portone-codex` through Codex's plugin manager using this repository's
[`portone` marketplace](../../.agents/plugins/marketplace.json).
Node.js and `npx` are required to run the bundled MCP server.

The plugin includes four skills: `portone-cli`, `portone-guide`,
`payment-code-generator`, and `integration-validator`.

## Usage

```text
Implement a PortOne V2 one-time payment integration.
Review the PortOne integration in this project.
Use the PortOne CLI to inspect failed test payments.
```

## CLI setup

You can also install the same skills and MCP settings with the
[PortOne CLI](../../README.md#installation):

```sh
portone setup --agent codex --scope user
portone setup update --agent codex --scope user
```

Restart Codex after setup. If you switch from the plugin to CLI setup, disable
the plugin to avoid duplicate skills and MCP servers.
See the [setup guide](../../docs/setup.md) for scopes and installation paths.

## Contributing

Edit the canonical skills under the repository's `skills/` directory and
[synchronize the plugin copies](../../docs/development.md).

## License

MIT
