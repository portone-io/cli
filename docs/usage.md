# Usage examples

English | [한국어](usage.ko.md)

Run `portone auth login` first. See [configuration](configuration.md) for
profiles and credentials, and the [command reference](reference/index.md)
for all options.

## Payments

```sh
portone payment list --test --status failed --limit 20
portone payment view payment-xxx
portone payment transactions payment-xxx
portone payment webhook list payment-xxx
```

Replace `payment-xxx` with your integration's payment ID. Use `list --search TEXT`
to search by PortOne or PG transaction ID. `transactions` uses an experimental API.

By default, `list` returns the newest 30 V2 payments changed within 90 days,
including test and live payments. It fetches pages automatically up to `--limit`
(1–60,000). Narrow the results with filters:

```sh
portone payment list --live --status paid,partial-cancelled --currency KRW
portone payment list --method card --pg tosspayments --version all
portone payment list --from 2026-09-01T00:00:00+09:00 --until 2026-09-08T00:00:00+09:00
```

For scripts, use JSON output and the embedded jq-compatible filter:

```sh
portone payment view payment-xxx --json
portone payment list --json id,status
portone payment list --json --jq '.[] | .id'
```

Lists produce arrays; other results produce objects. JSON preserves API field
names and values. Empty lists produce `[]` and succeed. Without `--json`, lists
use tables in a terminal and headerless TSV when piped. Amounts are integers
in the currency's minor unit.

## Cancellations and webhooks

```sh
portone payment cancel payment-xxx --reason 'Customer request'
portone payment cancel payment-xxx --amount 1000 --reason 'Partial refund' --yes
portone payment cancel payment-xxx --input cancel.json
portone payment webhook resend payment-xxx --webhook-id webhook-xxx
```

Cancellation requires a reason and prompts for confirmation. `--yes` skips the
prompt and is required without a TTY. Omitting `--amount` cancels the remaining
amount. All amount fields use integer minor currency units.

Use `--input FILE` or `--input -` for a complete JSON body, including a reason
and any refund account fields. It cannot be combined with individual cancellation
field flags. A body `storeId` overrides the default store but must match an
explicit `--store`.

`REQUESTED` means the cancellation was accepted; `SUCCEEDED` means it completed.
Both exit with code 0. `FAILED` exits with code 1. Cancellations are not retried
automatically.

Webhook resend runs without confirmation and selects the latest webhook when
`--webhook-id` is omitted. A successful request does not guarantee delivery;
reported delivery failures exit with code 1. Inspect request and response details
with `payment webhook list --json`.

## API requests

Use `portone api` for REST and GraphQL requests:

```sh
portone api /payments/payment-xxx
portone api /payments -X GET -F 'page[size]=10' -F 'filter[isTest]=true'
portone api /payments -X GET --paginate -q '.items[].id'
portone api graphql -f query='query { merchant { ... on Merchant { id plainId } } }'
```

The default method is GET; adding fields or `--input` switches to POST. Pass
`-X GET` when sending filters to a V2 list endpoint. `-f` sends strings; `-F`
converts integers, booleans, and null, and reads files with `@path` or stdin with `@-`.

`--paginate` fetches all pages. Use `--slurp` to collect them in an array, or
`--jq` to filter them. The embedded jaq engine supports most jq syntax;
external jq is optional.

`--cache 1h` caches eligible responses for one hour, excluding HTTP 403 and
5xx responses. The cache defaults to `~/.cache/portone`; override it with
`PORTONE_CACHE_DIR`. A custom `Authorization` header overrides saved credentials;
automatic credentials are omitted for full URLs on a different origin.

See [`portone api`](reference/portone_api.md) for nested fields, request bodies,
GraphQL variables, and pagination examples.

| Exit code | Meaning |
| --- | --- |
| 0 | Success, including an output pipe that closes early |
| 1 | HTTP 4xx/5xx, GraphQL errors, invalid flag combinations, or runtime errors |
| 2 | Command-line argument parsing error |

## Shell completion

Generate a script for Bash, Zsh, Fish, PowerShell, or Elvish:

```sh
portone completion zsh > "${fpath[1]}/_portone"
portone completion bash > "$(brew --prefix)/etc/bash_completion.d/portone"
portone completion fish > ~/.config/fish/completions/portone.fish
```

Use the command for your shell; the Bash example uses Homebrew's completion
directory. Open a new shell to enable `portone <TAB>` completion.
