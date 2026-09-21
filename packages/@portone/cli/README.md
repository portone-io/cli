# PortOne CLI

[English](README.md) | [한국어](README.ko.md)

[![npm version](https://img.shields.io/npm/v/%40portone%2Fcli)](https://www.npmjs.com/package/@portone/cli)
[![license](https://img.shields.io/github/license/portone-io/portone-cli)](https://github.com/portone-io/portone-cli/blob/main/LICENSE)

`portone` brings PortOne to the terminal. Search and cancel payments, manage
webhooks, and call the PortOne V2 API from the command line.

Supports macOS, Windows, and Linux.

## Installation

```sh
npm install --global @portone/cli
```

Requires Node.js 22.18.0 or later in the 22.x line, or Node.js 24 or later.

## Quick start

Log in through PortOne Console, then inspect payments:

```sh
portone auth login
portone payment list --test --status failed
portone payment view --payment-id payment-xxx
```

Replace `payment-xxx` with the `paymentId` set when making the payment.
Use `portone <command> --help` for options and examples.

## Documentation

- [Command reference](https://github.com/portone-io/portone-cli/blob/main/docs/reference/index.md)
- [Usage examples](https://github.com/portone-io/portone-cli/blob/main/docs/usage.md): payments, API requests, and shell completion
- [Configuration](https://github.com/portone-io/portone-cli/blob/main/docs/configuration.md): authentication, profiles, stores, and language

## Agent skills

Install PortOne skills and MCP settings for your coding agent:

```sh
portone setup

# Install for a specific agent and scope
portone setup --agent codex --scope user

# Update installed skills and MCP settings
portone setup update
```

Supports Claude Code, Codex, Cursor, Gemini CLI, GitHub Copilot CLI,
VS Code Copilot, and OpenCode. See the [setup guide](https://github.com/portone-io/portone-cli/blob/main/docs/setup.md)
for options and installation paths.

## Contributing

See the [development guide](https://github.com/portone-io/portone-cli/blob/main/docs/development.md) to build from source
and update documentation. Report bugs and request features in
[GitHub Issues](https://github.com/portone-io/portone-cli/issues).
