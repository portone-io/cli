# @portone/cli

## 0.2.0

### Minor Changes

- 3f81780: Add `portone api graphql` with GraphQL variables, cursor pagination, `--slurp`, and response caching. Report GraphQL errors and exit with status 1 even when the HTTP status is 200.
- 3f81780: Add PortOne Console OAuth browser login with automatic token refresh.

  - Store tokens in the OS keyring, with config-file fallback and a `--insecure-storage` option for explicit file storage.
  - Use Bearer tokens for REST and GraphQL authentication.
  - Add `portone auth token` to print the current access token.
  - Support `PORTONE_ACCESS_TOKEN` and reject `auth login` and `auth logout` while it is set.
  - Show the authentication method, access and session expiry, scopes, and issuing environment in `auth status`.
  - Preserve the authorization scheme when masking credentials in `--verbose` output.

- 3f81780: Add English and Korean support for help, prompts, authentication status, diagnostics, and login callback pages. Select the display language automatically from OS preferences, with overrides through `PORTONE_LANG` and the `language` setting in config.toml.
- 3f81780: Add `portone completion <shell>` to generate completion scripts for Bash, Zsh, Fish, PowerShell, and Elvish.

  Use the operating system trust store for TLS certificate verification.

- 3f81780: Update `portone setup` to install four official PortOne skills and MCP configuration for Claude Code, Codex, Cursor, Gemini CLI, GitHub Copilot CLI, VS Code Copilot, and OpenCode. Support multiple agents with `--agent` and project or user scope with `--scope`.

  Add `portone setup update` to refresh recorded installations from the latest official release independently of the CLI version, with agent/scope filters, `--dry-run`, and repair of missing or modified managed files.

  Remove `--assistant` and `--allow-dirty`. Require explicit `--agent` and `--scope` options for non-interactive setup.

  Enable bundled MCP tools for Claude agents.

- 3f81780: Require Node.js `^22.18.0 || >=24.0.0` for npm installations.
- 3f81780: Add `portone payment list`, `view`, `cancel`, `webhook list`, and `webhook resend` to search and inspect payments, cancel full or partial amounts, and inspect or resend webhooks.

  Support payment filters, automatic list pagination, store selection, readable output, and `--json` with field selection and `--jq` filtering. Require `--payment-id` for commands targeting a payment and `--yes` for non-interactive cancellation.

- 3f81780: Rewrite the CLI in Rust and distribute platform-specific native binaries.

  - Add `portone api <endpoint>` for PortOne V2 API requests with `-F`/`-f` fields, `--jq` filtering, pagination, `--slurp`, `--cache`, `--include`, and `--verbose`.
  - Add `portone auth login/status/logout` for authentication and profile management.

### Patch Changes

- 3f81780: Expand `portone api --help` and add the `portone-cli` skill for AI agents.
- dc0203a: Clarify one-time and billing-key payment terminology and reorganize payment-code-generator requirements gathering by feature, payment method, and nonstandard requirements.

## 0.1.0

### Minor Changes

- 9d0589d: Add Codex plugin support

## 0.0.2

### Patch Changes

- 192e025: Add a prompt to commit completed work to version control

## 0.0.1

### Patch Changes

- 9c2adc7: Release
