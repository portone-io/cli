---
"@portone/cli": minor
---

Rewrite the CLI in Rust and distribute platform-specific native binaries.

- Add `portone api <endpoint>` for PortOne V2 API requests with `-F`/`-f` fields, `--jq` filtering, pagination, `--slurp`, `--cache`, `--include`, and `--verbose`.
- Add `portone auth login/status/logout` for authentication and profile management.
