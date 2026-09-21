---
"@portone/cli": minor
---

Add PortOne Console OAuth browser login with automatic token refresh.

- Store tokens in the OS keyring, with config-file fallback and a `--insecure-storage` option for explicit file storage.
- Use Bearer tokens for REST and GraphQL authentication.
- Add `portone auth token` to print the current access token.
- Support `PORTONE_ACCESS_TOKEN` and reject `auth login` and `auth logout` while it is set.
- Show the authentication method, access and session expiry, scopes, and issuing environment in `auth status`.
- Preserve the authorization scheme when masking credentials in `--verbose` output.
