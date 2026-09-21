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

Use `--no-browser` to only print the login URL, or set the `PORTONE_BROWSER`
or `BROWSER` environment variable to choose a specific browser.

Tokens are stored in the OS keyring. If it is unavailable, the CLI warns and
uses the config file; `--insecure-storage` selects file storage explicitly.

## Profiles and stores

Use a profile for each merchant or environment:

```sh
portone auth login --profile staging
portone payment list --profile staging
portone store set-default --profile staging
portone store set-default --profile staging --view
portone store set-default --profile staging --unset
```

The config file is `~/.config/portone/config.toml` on Unix-like systems and
`%APPDATA%\portone\config.toml` on Windows. Set `PORTONE_CONFIG_DIR` to use
another directory.

```toml
language = "auto"
default_profile = "default"

[profiles.default]
base_url = "https://api.portone.io"
store_id = "store-xxx"
```

Settings use the following precedence:

| Setting | Precedence |
| --- | --- |
| Credentials | `PORTONE_ACCESS_TOKEN`, profile |
| Profile | `--profile`, `default_profile`, `default` |
| API base URL | `--base-url`, `PORTONE_API_BASE`, profile `base_url`, `https://api.portone.io` |
| Store | `--store`, `PORTONE_STORE_ID`, profile `store_id`, API default |

When the `PORTONE_ACCESS_TOKEN` environment variable is set, saved login
credentials are ignored. Unset it before running `auth login` or `auth logout`.
`payment list --all-stores` ignores the default store and cannot be combined
with `--store`.

## Display language

The CLI supports English and Korean. Use the `PORTONE_LANG` environment variable
to set the display language manually:

```sh
PORTONE_LANG=en portone auth status
PORTONE_LANG=ko portone --help
```

Display language uses the following precedence:

| Setting | Precedence |
| --- | --- |
| Display language | `PORTONE_LANG`, config file `language`, OS language detection, English |
| Linux language detection | `LANGUAGE`, `LC_ALL`, `LC_MESSAGES`, `LANG` |

`auto` uses OS language detection. `PORTONE_LANG=auto` also overrides the config
file's `language`. On macOS and Windows, detection uses UI language preferences.
Regional locales such as `ko-KR` are accepted, and unsupported languages fall back
to English.

See the [command reference](reference/index.md) for all options.
