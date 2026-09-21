---
"@portone/cli": minor
---

Add `portone payment list`, `view`, `cancel`, `webhook list`, and `webhook resend` to search and inspect payments, cancel full or partial amounts, and inspect or resend webhooks.

Support payment filters, automatic list pagination, store selection, readable output, and `--json` with field selection and `--jq` filtering. Require `--payment-id` for commands targeting a payment and `--yes` for non-interactive cancellation.
