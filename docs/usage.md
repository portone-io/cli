# Usage examples

English | [한국어](usage.ko.md)

Run `portone auth login` first. See [configuration](configuration.md) for
profiles and credentials, and the [command reference](reference/index.md)
for all options.

## Payments

```sh
portone payment list --test --status failed --limit 20
portone payment view --payment-id payment-xxx
portone payment webhook list --payment-id payment-xxx
```

Replace `payment-xxx` with the `paymentId` set when making the payment.
Use `list --search TEXT` to search payments.

By default, `list` returns the newest 30 payments changed within 90 days.
Use `--limit` (1–60,000) to specify how many payments to retrieve.

```sh
portone payment list --live --status paid,partial-cancelled --currency KRW
portone payment list --method card --pg tosspayments --version all
portone payment list --from 2026-09-01T00:00:00+09:00 --until 2026-09-08T00:00:00+09:00
```

You can use JSON output and jq filters:

```sh
portone payment view --payment-id payment-xxx --json
portone payment list --json id,status
portone payment list --json --jq '.[] | .id'
```

Without `--json`, output uses tables in a terminal and headerless TSV when piped.
Amounts are integers in the currency's minor unit (for example, 1 USD = 100, 1 KRW = 1).

## Cancellations and webhooks

```sh
portone payment cancel --payment-id payment-xxx --reason 'Customer request'
portone payment cancel --payment-id payment-xxx --amount 1000 --reason 'Partial refund' --yes
portone payment cancel --payment-id payment-xxx --input cancel.json
portone payment webhook resend --payment-id payment-xxx --webhook-id webhook-xxx
```

Cancellation requires a reason and prompts for confirmation. `--yes` skips the
prompt and is required in non-interactive environments. Omitting `--amount`
cancels the remaining amount. All amount fields use integer minor currency units.

Use `--input FILE` or `--input -` for a complete JSON body, including a reason
and any refund account fields.

`REQUESTED` means the cancellation was accepted; `SUCCEEDED` means it completed.
Both exit with code 0. `FAILED` exits with code 1.

Webhook resend runs without confirmation and selects the latest webhook when
`--webhook-id` is omitted. A successful request does not guarantee delivery;
reported delivery failures exit with code 1. Inspect request and response details
with `payment webhook list --payment-id payment-xxx --json`.

## API requests

Use `portone api` for REST and GraphQL requests:

```sh
portone api /payments/payment-xxx
portone api /payments -X GET -F 'page[size]=10' -F 'filter[isTest]=true'
portone api /payments -X GET --paginate -q '.items[].id'
portone api graphql -f query='query { merchant { ... on Merchant { id plainId } } }'
```

The default method is GET; adding fields or `--input` switches to POST. Pass
`-X GET` when sending filters to a list endpoint. `-f` sends strings; `-F`
converts integers, booleans, and null, and reads files with `@path` or stdin with `@-`.

`--paginate` fetches all pages. Use `--slurp` to collect them in an array, or
`--jq` to filter them.

`--cache 1h` caches eligible responses for one hour, excluding HTTP 403 and
5xx responses. The cache defaults to `~/.cache/portone`; override it with
`PORTONE_CACHE_DIR`.

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
