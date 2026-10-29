# Quotes

A quote linked to the deal, with versions, line items with discount and tax, approval, sending through the user's mailbox and acceptance through the public link.

## Where it lives

| Layer | Path |
| --- | --- |
| Domain (states, items, totals calculation) | `back/modules/quotes/src/domain.rs` |
| Use cases | `back/modules/quotes` (`use_cases.rs` CRUD/versions/settings, `workflow.rs` approval/sending/expiration, `public.rs` public link, `sync.rs` acceptance → deal, `pricing.rs` item pricing, `runtime.rs` jobs and consumer) |
| MySQL | `back/modules/quotes/src/store.rs` (tables `quotes`, `quote_items`, `quote_counters`, `quote_settings`, migrations `2510`–`2514`) |
| API | `back/modules/quotes/src/http.rs` (`/quotes`, `/deals/{id}/quotes`, `/quote-settings` and the public page `/public/quotes/{token}`) |
| Web | `web/src/features/quotes` (tab on the deal detail, editor `/quotes/:id`, list `/quotes`, settings `/settings/quotes`) and `web/src/public/quotes` (public page `/quote/:token`) |

## Record type

`quote` is in the `RecordTypeRegistry` (`modules/quotes/mod.rs`): listing, filtering, sorting and search through the `RecordQueryEngine`, custom fields through the `CustomFieldWriter`, the record pipeline (`fields().pipeline()`: `saving`, `validate`, `saved`, `committed`) on create, edit and new version. Owner = `owner_id` (permissions `quotes.read|create|update|delete|send`, by scope). Each version is a record; `is_current` marks the current version and the default listing filters by it (`include_versions=true` shows all of them).

System field `exceeds_discount_limit`: true when the discount of any line item or of the total, as an effective percentage (an amount discount is also converted to %), exceeds the limit of the owner's role (`DiscountLimits::max_discount_percent`; no limit = false). It is recalculated on every write and before submitting for approval or sending to the customer; the default `quote_discount_limit` policy uses this field.

## Number and versions

The number comes from `quote_counters` (a single row incremented with `LAST_INSERT_ID` inside the transaction, with no race). A new version (`POST /quotes/{id}/versions`) copies the header, line items and custom fields with the same number and `version + 1`; previous versions become `is_current = false` and read-only (409 `quote_not_current`) and their open approval requests are cancelled (`ApprovalGate::cancel_open`). The unique index `(current_number)` guarantees a single current version.

## Totals (server-side, `rust_decimal`, 2 decimal places, round half away from zero)

1. Line gross = quantity × unit price.
2. Line discount (percentage or amount, capped at the gross); net = gross − discount.
3. Total discount (percentage or amount) over the sum of the nets, **prorated** across the lines in proportion to the net (the last line with an amount absorbs the rounding).
4. Line tax = the line's % over (net − share of the total discount).
5. Line total = taxable base + tax; quote total = sum of the lines. `discount_total` = line discounts + total discount.

Catalog items without a given price use the quote's price list (`PriceListResolver::price`), falling back to the product price when the currency matches; bundles (`BundleExpander`) come in as one line with the bundle price and the components stored for display. `reprice: true` (or changing the list) reprices the product lines. The currency is the deal's at creation time and does not change.

## States

`draft` → `pending_approval` → `approved` → `sent` → `accepted` | `rejected` | `expired`.

- **Submit for approval** (`submit`, `draft` only): `ApprovalGate::submit(quote_send)` in the transaction; `Pending` → `pending_approval`, `NotRequired`/`Approved` → `approved`. The status is written directly in the repository (bypassing the guard, which would block the record with a pending request).
- The `quotes.approvals` consumer (`approval.approved|rejected|cancelled` with `record_type = quote`) moves `pending_approval` → `approved` (when no pending or rejected request remains) or → `draft` (a rejection cancels the other open requests).
- **Edit** only in `draft`/`approved`; editing an `approved` quote moves it back to `draft` and calls `cancel_open` (the previous approval does not apply to the new content). In `pending_approval` → 409 `approval_pending`.
- **Send** (`send`, from `draft`, `approved` or resending a `sent` one): calls `ApprovalGate::checkpoint(quote_send)` (consumes the approval; 409 `approval_required` when one is needed and missing — a quote with a discount above the limit does not go out before approval), writes `sent_at`, publishes `quote.sent` and `deal.quote_sent` and sends the email through the `email` module's `email::Sender` service (the user's mailbox, linked to the deal) with the transaction still open: if sending fails, nothing is saved.
- **Record acceptance** (`POST /quotes/{id}/accept`, `quotes.update`, from `draft`, `approved` or `sent`): acceptance received outside the link (signed paper, phone, email). It goes through the same `ApprovalGate::checkpoint(quote_send)` as sending when the quote has not been sent, runs the `QuoteAccepting` hook, saves name (default: the quote's contact), email (default: the contact's email), IP and `accepted_by` (the user who recorded it; `acceptance.recorded_by` in the API), publishes `quote.accepted` with `manual: true`, `recorded_by` and `note`, and `deal.quote_accepted` with the user as actor, and enqueues the same `quote.deal_sync` as the public acceptance.
- **Expire**: the hourly `quotes.expire` job marks `sent` quotes whose `valid_until` is before today in the organization's time zone.

## Public link

A random `public_key` (16 bytes) per sent version; the token is `base64url(public_key + HMAC-SHA256[..16])` with the organization's key in the `SecretVault` (`quote_links`/`token_key`). It does not expose internal ids and only shows the sent version; a superseded version appears as `superseded`. Public routes with per-IP and per-link limits; `csrf.rs` allows `/api/v1/public/quotes/`.

## Acceptance → deal

`POST /public/quotes/{token}/accept` saves name, email, IP, user agent and date, publishes `quote.accepted`/`deal.quote_accepted` and enqueues `quote.deal_sync` in the same transaction. The job (idempotent, acting with the scope of the quote's owner via `QuoteGateway::member_scope`) uses the deal items and deals use cases:

- replaces the deal's line items with the lines of the accepted version (line discount + share of the total discount become an amount discount; tax does not exist on the items);
- amount: in manual mode the deal receives the quote total (with taxes); in "amount = sum of items" mode it keeps the sum of the items (total without taxes);
- if `quote_settings.accept_stages[pipeline]` is configured and the deal is open, it moves to that stage. A business failure (approval, stage requirements, validation) does not undo the acceptance: it is kept in `deal_sync_error` and goes out in `quote.deal_sync_failed`; success publishes `quote.deal_synced`.

## PDF

`GET /quotes/{id}/pdf` (`quotes.read`) generates the version's PDF on the fly (without saving a file): header with organization, number/version and date, status, company, contact, owner, validity, acceptance, line items table, subtotal, discounts, taxes, total, terms and notes. The generator (`pdf.rs`, Helvetica/WinAnsi, no dependencies) is an adapted copy of the one in `proposals` (which depends on `quotes`, so there is no direct reuse). Totals hidden by field permission come out as "—". The editor always has "Download PDF"; "Generate PDF" with a template still goes through `proposals`.

## Optional modules

- `proposals`: `quote_id` in `POST /api/v1/proposals/preview` and `/deals/{id}/documents` makes the quote the data source (`quote.*` variables, the quote's line items table and totals), read through `host.api_get("/v1/quotes/{id}")`.
- `esign`: optional `quote_id` in `POST /api/v1/esign/requests` (validated against the deal and included in `esign.sent`). In the editor, "Send for signature" generates the PDF through `proposals` and sends it through `esign`; it only appears with the module enabled (without it, the routes respond 404 `module_disabled`).
- Without `proposals`, the public page has a print version (Tailwind's `print:` and a print button).

## Events

`quote.created|updated|deleted|version_created|submitted|approved|returned_to_draft|sent|viewed|accepted|rejected|expired|deal_synced|deal_sync_failed` (with `changes` on edits and `records: [deal]` in the payload) and, on the deal timeline, `deal.quote_sent|quote_accepted|quote_rejected`. The `quote.*` events are in the public catalog with the `QuoteDetailResponse` resource.
