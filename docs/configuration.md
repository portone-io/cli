# Configuration

English | [한국어](configuration.ko.md)

## Authentication

Log in through PortOne Console:

```sh
portone auth login
portone auth status
portone auth token   # Print the access token, refreshing if needed
portone auth logout  # Remove local credentials
```

Login opens a browser and waits up to five minutes for a callback on
`127.0.0.1:1271`. Use `--no-browser` to print the URL, or `PORTONE_BROWSER`
or `BROWSER` to choose a browser.

Tokens are stored in the OS keyring. If it is unavailable, the CLI warns and
uses the config file; `--insecure-storage` selects file storage explicitly.
Access tokens refresh automatically. Refresh tokens rotate on use and expire
after 24 hours of inactivity. Log in separately on each machine, and log in
again when the session expires. Logout removes local credentials without
revoking the token.

## Profiles and stores

Use a profile for each merchant or environment:

```sh
portone auth login --profile staging
portone payment list --profile staging
portone store set-default --profile staging
portone store set-default --profile staging --view
portone store set-default --profile staging --unset
```

Login selects the representative store by default and retains an accessible
store selection on later logins. If there is no representative store, it selects
the only accessible store or prompts when several exist. Store selection only
changes the CLI profile.

The config file is `~/.config/portone/config.toml` on Unix-like systems and
`%APPDATA%\portone\config.toml` on Windows. Set `PORTONE_CONFIG_DIR` to use
another directory. Login manages the authentication entries; other settings include:

```toml
language = "auto"
default_profile = "default"

[profiles.default]
base_url = "https://api.portone.io"
store_id = "store-xxx"
```

Settings use the first available value in these lists:

| Setting | Precedence |
| --- | --- |
| Credentials | `PORTONE_ACCESS_TOKEN`, selected OAuth profile |
| Profile | `--profile`, `default_profile`, `default` |
| API base URL | `--base-url`, `PORTONE_API_BASE`, profile `base_url`, `https://api.portone.io` |
| Store | `--store`, `PORTONE_STORE_ID`, profile `store_id`, API default |

`PORTONE_ACCESS_TOKEN` is used as provided and never refreshed. Unset it before
running `auth login` or `auth logout`. `payment list --all-stores` ignores the
default store and cannot be combined with `--store`.

For a custom login environment, use `PORTONE_CONSOLE_URL`,
`PORTONE_MERCHANT_SERVICE_URL`, `PORTONE_OAUTH_CLIENT_ID`, and
`PORTONE_OAUTH_REDIRECT_URI`. These apply only during login.

## Display language

The CLI supports English and Korean. Set `PORTONE_LANG` for one process, or
save `language = "en"`, `"ko"`, or `"auto"` in the config file:

```sh
PORTONE_LANG=en portone auth status
PORTONE_LANG=ko portone --help
```

`PORTONE_LANG` overrides the saved preference; `auto` uses OS language detection.
On macOS and Windows, detection uses UI language preferences. On Linux, it checks
`LANGUAGE`, `LC_ALL`, `LC_MESSAGES`, and `LANG` in order. Regional locales such
as `ko-KR` are accepted, and unsupported languages fall back to English.

Command names, flags, and API responses retain their original values. Argument
parsing errors and generated completion scripts use English. For consistent CI
or agent diagnostics, set `PORTONE_LANG=en`. In PowerShell, use
`$env:PORTONE_LANG = 'en'` before running the CLI.

See the [command reference](reference/index.md) for all options.
