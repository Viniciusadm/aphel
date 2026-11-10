---
title: Webhooks and events
description: The public event catalog, the envelope of delivered events, outgoing webhook subscriptions with signatures and retries, the network policy, incoming webhooks and plugin events.
sidebar:
  order: 6
---

Aphel publishes a domain event for every business change. A stable subset, the **public events**, can be delivered to your systems as signed HTTP requests (**outgoing webhooks**), and other systems can push data in through secret URLs (**incoming webhooks**). The webhook features belong to the `webhooks` module; the event catalog route is part of the platform.

## The event envelope

Every delivered event has the same envelope:

```json
{
  "id": "0192f0c4-8c1e-7d2a-9f3b-1c2d3e4f5a6b",
  "type": "deal.stage_changed",
  "actor": { "type": "user", "id": "0192a1b2-0000-7000-8000-0000000000aa" },
  "occurred_at": "2026-10-20T13:45:00.123Z",
  "data": { "id": "0192a1b2-0000-7000-8000-0000000000d1", "title": "Acme onboarding", "stage_id": "0192a1b2-0000-7000-8000-0000000000e2" },
  "previous": { "stage_id": "0192a1b2-0000-7000-8000-0000000000e1" }
}
```

| Property | Meaning |
| --- | --- |
| `id` | Unique id of the event. It stays the same across retries and redeliveries, so use it to deduplicate. |
| `type` | The event type, such as `deal.stage_changed`. |
| `actor` | Who caused the change. `type` is `user`, `api_token`, `system`, `automation` or `plugin`, and `id` identifies the user, token, automation or plugin (`null` for `system`). |
| `occurred_at` | When the change happened, in UTC with millisecond precision. |
| `data` | The resource after the change, in the same shape as `GET /{resource}/{id}`. For deletions only `{ "id": "..." }`. Events that are not about a record carry their own payload. |
| `previous` | Only when the event type defines it: the old values of the fields that changed. Keys absent from `previous` did not change. |

Fields the receiving integration should not see are never included: payload keys that look like secrets (names containing `secret`, `password`, `token`, `credential` or `api_key`) are removed, and internal bookkeeping keys are not exposed. Events are delivered at least once and in no guaranteed order; use `occurred_at` and the version (`updated_at`) of `data` to order them.

## Listing event types

`GET /api/v1/events/catalog` returns the catalog of the build, readable by any authenticated member:

```json
{
  "data": [
    {
      "type": "deal.stage_changed",
      "resource": "deals.deal",
      "description": "A deal moved to another stage",
      "has_previous": true,
      "data_is_id_only": false,
      "data_schema": "#/components/schemas/DealResponse"
    }
  ]
}
```

`data_schema` points to the OpenAPI schema of `data`, `has_previous` tells whether `previous` is sent and `data_is_id_only` whether `data` is only `{ "id": ... }`. The list also contains the events of each active custom object. It is the source of truth for what you can subscribe to in your installation; the tables below show the events of the modules built into the binary.

## Event catalog

Events appear in the catalog only while their module is enabled. Resource names are record types or payload names.

### Platform

| Event | Resource | Description |
| --- | --- | --- |
| `organization.updated` | `organization` | The organization settings changed (`data` has `changed_fields` and the current `currency`; `previous_currency` when the base currency changed) |

### Approvals (`approvals`)

| Event | Resource | Description |
| --- | --- | --- |
| `approval.requested` | `deals.deal` | An approval was requested for the deal (stage change or on demand) |
| `approval.approved` | `deals.deal` | Every step of an approval request of the deal was approved |
| `approval.rejected` | `deals.deal` | An approval request of the deal was rejected |
| `approval.cancelled` | `deals.deal` | An approval request of the deal was cancelled |

### Booking (`booking`)

| Event | Resource | Description |
| --- | --- | --- |
| `meeting.booked` | `people.person` | The contact booked a meeting through a booking link |
| `meeting.rescheduled` | `people.person` | A meeting booked by the contact through a booking link was rescheduled |
| `meeting.canceled` | `people.person` | A meeting booked by the contact through a booking link was canceled |

### Calls (`telephony`)

| Event | Resource | Description |
| --- | --- | --- |
| `call.logged` | `tasks.task` | A call was logged as an activity (details in `call`: direction, outcome, duration) |
| `call.updated` | `tasks.task` | The details of a logged call changed |

### CNPJ enrichment (`enrichment`)

| Event | Resource | Description |
| --- | --- | --- |
| `company.enriched` | `organizations.organization` | Fields of the company were filled from a public CNPJ lookup |

### Deals (`deals`)

| Event | Resource | Description |
| --- | --- | --- |
| `deal.created` | `deals.deal` | A deal was created |
| `deal.updated` | `deals.deal` | Fields of a deal changed (stage, pipeline and status have their own events) |
| `deal.stage_changed` | `deals.deal` | A deal moved to another stage |
| `deal.pipeline_changed` | `deals.deal` | A deal moved to another pipeline |
| `deal.amount_changed` | `deals.deal` | The amount of a deal changed |
| `deal.owner_changed` | `deals.deal` | The owner of a deal changed |
| `deal.expected_close_date_changed` | `deals.deal` | The expected close date of a deal changed |
| `deal.won` | `deals.deal` | A deal was won |
| `deal.lost` | `deals.deal` | A deal was lost |
| `deal.reopened` | `deals.deal` | A won or lost deal was reopened |
| `deal.deleted` | `deals.deal` | A deal was moved to the trash |
| `deal.archived` | `deals.deal` | A deal was archived |
| `deal.unarchived` | `deals.deal` | A deal was unarchived |
| `deal.member_added` | `deals.deal` | A user joined the team of a deal (`previous` has `member.user_id`, `member.role` and `member.credit_percent` as null; the team is at `GET /deals/{id}/members`) |
| `deal.member_updated` | `deals.deal` | The role or revenue credit of a member of a deal changed |
| `deal.member_removed` | `deals.deal` | A user left the team of a deal (`previous` has the user, role and credit of the member) |

### Electronic signature (`esign`)

| Event | Resource | Description |
| --- | --- | --- |
| `esign.queued` | `deals.deal` | A signature request was queued for delivery |
| `esign.cancel_queued` | `deals.deal` | A signature cancellation was queued |
| `esign.failed` | `deals.deal` | A signature delivery could not be confirmed |
| `esign.sent` | `deals.deal` | A document of the deal was sent for electronic signature |
| `esign.signed` | `deals.deal` | A signer signed a document of the deal |
| `esign.refused` | `deals.deal` | A signer refused a document of the deal |
| `esign.cancelled` | `deals.deal` | A signature request of the deal was cancelled |
| `esign.completed` | `deals.deal` | Every signer signed a document of the deal |
| `esign.expired` | `deals.deal` | A signature request of the deal expired |
| `esign.signed_file_attached` | `deals.deal` | The signed file was attached to the deal |

### Lead forms (`forms`)

| Event | Resource | Description |
| --- | --- | --- |
| `lead_form.submitted` | `people.person` | A lead form was submitted, creating or updating the contact |

### Leads (`leads`)

| Event | Resource | Description |
| --- | --- | --- |
| `lead.qualified` | `people.person` | A lead was qualified in the lead inbox (the contact moved from `lead` to `qualified`) |
| `lead.disqualified` | `people.person` | A lead was disqualified in the lead inbox with a reason |
| `lead.converted` | `people.person` | A lead was converted: a deal was created with the contact and the contact moved to the converted stage |

### Money (`money`)

| Event | Resource | Description |
| --- | --- | --- |
| `exchange_rate.created` | `money.exchange_rate` | An exchange rate was registered, by hand or fetched from the configured provider |
| `exchange_rate.updated` | `money.exchange_rate` | An exchange rate changed (edited by hand or refreshed by the provider) |
| `exchange_rate.deleted` | `money.exchange_rate` | An exchange rate was removed |
| `currency.enabled` | `currency` | A currency was enabled for the organization (`data` has `currency` and `base`) |
| `currency.disabled` | `currency` | A currency was disabled for the organization (`data` has `currency` and `base`) |
| `exchange_rate.rebased` | `currency_base` | The base currency of the organization changed and the stored rates were converted to it |

### Notes (`notes`)

| Event | Resource | Description |
| --- | --- | --- |
| `note.created` | `notes.note` | A note was created |
| `note.updated` | `notes.note` | A note was edited |
| `note.deleted` | `notes.note` | A note was deleted |

### Organizations (`organizations`)

| Event | Resource | Description |
| --- | --- | --- |
| `company.created` | `organizations.organization` | A company was created |
| `company.updated` | `organizations.organization` | Fields of a company changed |
| `company.deleted` | `organizations.organization` | A company was moved to the trash |
| `company.archived` | `organizations.organization` | A company was archived |
| `company.unarchived` | `organizations.organization` | A company was unarchived |
| `company.merged` | `organizations.organization` | Another company was merged into this one |

### People (`people`)

| Event | Resource | Description |
| --- | --- | --- |
| `contact.created` | `people.person` | A contact was created |
| `contact.updated` | `people.person` | Fields of a contact changed |
| `contact.deleted` | `people.person` | A contact was moved to the trash |
| `contact.archived` | `people.person` | A contact was archived |
| `contact.unarchived` | `people.person` | A contact was unarchived |
| `contact.merged` | `people.person` | Another contact was merged into this one |

### Price lists (`pricing`)

| Event | Resource | Description |
| --- | --- | --- |
| `price_list.created` | `pricing.price_list` | A price list was created |
| `price_list.updated` | `pricing.price_list` | Fields of a price list changed |
| `price_list.deleted` | `pricing.price_list` | A price list was removed |
| `price_list.items_replaced` | `pricing.price_list` | The prices of a price list were replaced (items at `GET /pricing/{id}/items`) |
| `product_bundle.updated` | `catalog_items.item` | The bundle of a product was saved (components at `GET /pricing/items/{id}/bundle`) |
| `product_bundle.removed` | `catalog_items.item` | A product stopped being a bundle |

### Products (`catalog_items`)

| Event | Resource | Description |
| --- | --- | --- |
| `product.created` | `catalog_items.item` | A product was added to the catalog |
| `product.updated` | `catalog_items.item` | Fields of a product changed |
| `product.deleted` | `catalog_items.item` | A product was removed from the catalog |

### Proposals (`proposals`)

| Event | Resource | Description |
| --- | --- | --- |
| `proposal.generated` | `deals.deal` | A proposal PDF was generated and attached to the deal |

### Quotes (`quotes`)

| Event | Resource | Description |
| --- | --- | --- |
| `quote.created` | `quotes.quote` | A quote was created for a deal |
| `quote.updated` | `quotes.quote` | Fields or items of a quote changed |
| `quote.deleted` | `quotes.quote` | A quote version was deleted |
| `quote.version_created` | `quotes.quote` | A new version of a quote was created from the current one |
| `quote.submitted` | `quotes.quote` | A quote was sent for approval |
| `quote.approved` | `quotes.quote` | A quote was approved |
| `quote.returned_to_draft` | `quotes.quote` | An approval of the quote was rejected or cancelled and it went back to draft |
| `quote.sent` | `quotes.quote` | A quote was sent to the customer by e-mail with its public link |
| `quote.viewed` | `quotes.quote` | The customer opened the public link of the quote for the first time |
| `quote.accepted` | `quotes.quote` | The quote was accepted: by the customer through its public link, or recorded manually by a user (`manual: true`) |
| `quote.rejected` | `quotes.quote` | The customer rejected the quote through its public link |
| `quote.expired` | `quotes.quote` | A sent quote passed its validity date |
| `quote.deal_synced` | `quotes.quote` | The deal received the items and the amount of the accepted quote |
| `quote.deal_sync_failed` | `quotes.quote` | The deal could not be fully updated after the quote was accepted |

### Renewals (`renewals`)

| Event | Resource | Description |
| --- | --- | --- |
| `deal.renewal_created` | `deals.deal` | A renewal deal was created for a won deal whose contract is ending (the body is the renewal; `renewal_of_deal_id` points to the original) |

### Scoring (`scoring`)

| Event | Resource | Description |
| --- | --- | --- |
| `score.band_changed` | `people.person` | The score of the contact crossed into another band (cold, warm, hot) of a scoring model |
| `score.company_band_changed` | `organizations.organization` | The score of the company crossed into another band (cold, warm, hot) of a scoring model |

### Sequences (`sequences`)

| Event | Resource | Description |
| --- | --- | --- |
| `sequence.enrolled` | `people.person` | A contact was enrolled in a sequence |
| `sequence.completed` | `people.person` | A contact went through every step of a sequence |
| `sequence.exited` | `people.person` | A contact left a sequence (reply, meeting, deal won or lost, bounce, objection or removal) |

### Tasks (`tasks`)

| Event | Resource | Description |
| --- | --- | --- |
| `activity.created` | `tasks.task` | An activity was created |
| `activity.updated` | `tasks.task` | Fields of an activity changed |
| `activity.completed` | `tasks.task` | An activity was completed |
| `activity.reopened` | `tasks.task` | A completed activity was reopened |
| `activity.deleted` | `tasks.task` | An activity was deleted |

### Workflow (`workflow`)

| Event | Resource | Description |
| --- | --- | --- |
| `workflow.checklist_item_checked` | `workflow.record` | An item of the stage checklist of the record was checked |
| `workflow.checklist_item_unchecked` | `workflow.record` | An item of the stage checklist of the record was unchecked |

### Custom objects

Each active custom object publishes these events, with `custom.<key>` replaced by the record type (for example `custom.contract.created`). `data` has the shape of a custom record response.

| Event | Description |
| --- | --- |
| `custom.<key>.created` | A record of a custom object was created. |
| `custom.<key>.updated` | Fields of a record of a custom object changed. `previous` has the old values. |
| `custom.<key>.deleted` | A record of a custom object was deleted. |
| `custom.<key>.archived` | A record of a custom object was archived. |
| `custom.<key>.unarchived` | A record of a custom object was unarchived. |

### Plugin events

A runtime plugin that lists an event in `[[emits]]` of its manifest can publish `plugin.<key>.<name>` with the host `emit` call. The webhooks module recognizes every `plugin.*` event type as a public event. Plugin events are also automation triggers. How to declare and emit them is described in [Events](/_aphel/docs/plugins/events/).

## Outgoing webhooks

An **endpoint** is a URL plus the list of event types it receives. Managing endpoints needs the permission `settings.manage`.

| Route | Purpose |
| --- | --- |
| `GET /api/v1/webhook-endpoints` | List endpoints, newest first. |
| `POST /api/v1/webhook-endpoints` | Create one. |
| `GET`, `PATCH`, `DELETE /api/v1/webhook-endpoints/{endpoint_id}` | Read, change (`url`, `events`, `description`, `is_active`) or remove. |
| `POST /api/v1/webhook-endpoints/{endpoint_id}/rotate-secret` | Generate a new signing secret. |
| `POST /api/v1/webhook-endpoints/{endpoint_id}/test` | Send a synthetic `webhook.test` event right away. |
| `GET /api/v1/webhook-endpoints/{endpoint_id}/deliveries` | The delivery log of the endpoint. |
| `GET /api/v1/webhook-endpoints/{endpoint_id}/deliveries/{delivery_id}` | One delivery with its attempts. |
| `POST /api/v1/webhook-endpoints/{endpoint_id}/deliveries/{delivery_id}/redeliver` | Send a delivery again. |

### Creating an endpoint

```bash
curl -X POST "$APHEL_URL/api/v1/webhook-endpoints" \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com/aphel/webhooks",
    "events": ["deal.won", "deal.lost", "contact.created"],
    "description": "ERP sync"
  }'
```

`events` is a list of event types from the catalog, or `["*"]` for every event. The `201` response is the endpoint plus `signing_secret`, which starts with `whsec_` and is shown only in this response and in the response of `rotate-secret`:

```json
{
  "data": {
    "id": "0192a1b2-0000-7000-8000-000000000001",
    "url": "https://example.com/aphel/webhooks",
    "description": "ERP sync",
    "events": ["deal.won", "deal.lost", "contact.created"],
    "is_active": true,
    "consecutive_failures": 0,
    "secret_configured": true,
    "disabled_at": null,
    "disabled_reason": null,
    "last_delivery_at": null,
    "last_success_at": null,
    "created_at": "2026-10-21T13:45:00.000Z",
    "updated_at": "2026-10-21T13:45:00.000Z",
    "signing_secret": "whsec_9c1e0d7a4b2f6e8d0a1b3c5d7e9f1a2b4c6d8e0f1a3b5c7d9e1f2a4b6c8d0e2f4a"
  }
}
```

Limits: 20 endpoints per installation (`422 too_many_webhook_endpoints`), URLs up to 2048 characters and descriptions up to 255. Signing secrets are stored encrypted with `APP_ENCRYPTION_KEY`; without it, creating an endpoint answers `503`. Rotating a secret takes effect immediately, also for retries that are still pending.

### What Aphel sends

Each delivery is a `POST` with `Content-Type: application/json`, the event envelope as body, and these headers:

| Header | Value |
| --- | --- |
| `X-Aphel-Event` | The event type. |
| `X-Aphel-Delivery` | The delivery id. It is the same on every attempt of that delivery. |
| `X-Aphel-Signature` | `t=<unix seconds>,v1=<hex>`, described below. |
| `User-Agent` | `Aphel/1.0` |

A delivery succeeds when your endpoint answers with a `2xx` status within the timeout. Redirects are not followed and count as failures, so register the final URL.

### Verifying the signature

`v1` is the lowercase hexadecimal HMAC-SHA256, keyed with the endpoint signing secret (the full `whsec_...` string), of the text `<t>.<raw request body>`: the timestamp from the header, a dot, and the exact bytes of the body. Compute it over the raw body, before any JSON parsing, compare in constant time and reject timestamps that are too old to protect against replays.

```javascript
import { createHmac, timingSafeEqual } from 'node:crypto'

export function verifyAphelSignature(secret, header, rawBody, toleranceSeconds = 300) {
  const parts = Object.fromEntries(
    header.split(',').map((part) => part.trim().split('=', 2)),
  )
  const timestamp = Number(parts.t)
  if (!Number.isFinite(timestamp) || !parts.v1) return false
  if (Math.abs(Date.now() / 1000 - timestamp) > toleranceSeconds) return false
  const expected = createHmac('sha256', secret)
    .update(`${timestamp}.${rawBody}`)
    .digest()
  const received = Buffer.from(parts.v1, 'hex')
  return received.length === expected.length && timingSafeEqual(received, expected)
}
```

```python
import hashlib
import hmac
import time


def verify_aphel_signature(secret: str, header: str, raw_body: bytes, tolerance: int = 300) -> bool:
    parts = dict(item.strip().split("=", 1) for item in header.split(","))
    try:
        timestamp = int(parts["t"])
        received = bytes.fromhex(parts["v1"])
    except (KeyError, ValueError):
        return False
    if abs(time.time() - timestamp) > tolerance:
        return False
    expected = hmac.new(
        secret.encode(), str(timestamp).encode() + b"." + raw_body, hashlib.sha256
    ).digest()
    return hmac.compare_digest(received, expected)
```

### Retries and backoff

A failed event delivery is retried automatically. An event delivery makes at most 7 attempts: the first one and 6 retries, spaced like this after each failure.

| After failed attempt | Wait before the next |
| --- | --- |
| 1 | 1 minute |
| 2 | 5 minutes |
| 3 | 30 minutes |
| 4 | 2 hours |
| 5 | 6 hours |
| 6 | 12 hours |

After the 7th failure the delivery is marked `failed` and stays in the log. Test deliveries and redeliveries make a single attempt.

Delivery settings are part of the organization settings (`GET` and `PATCH /api/v1/organization/settings`, under the `webhooks` key):

| Setting | Default | Range | Meaning |
| --- | --- | --- | --- |
| `timeout_secs` | 10 | 1 to 120 | How long to wait for your answer. |
| `require_https` | `false` | | When true, only `https` URLs are accepted. |
| `disable_after_failures` | 15 | 1 to 1000 | Consecutive failed deliveries (after exhausting their retries) that disable the endpoint automatically. |
| `retention_days` | 30 | 1 to 3650 | How long the delivery log is kept. |

When an endpoint is disabled (`disabled_reason` is `failures` or `manual`), it stops receiving events and `redeliver` answers `409 webhook_endpoint_disabled`. Re-enable it with `PATCH` and `"is_active": true`. The log of each delivery keeps the status (`pending`, `succeeded`, `failed`), the number of attempts, the response status, up to 2048 bytes of the response body, the error and the duration. A delivery whose `trigger` is `event` came from a real event; `test` and `redelivery` were requested by a user. A redelivery keeps the same event `id` and body and is signed with the current secret.

Your receiver should answer quickly with a `2xx` and process the event afterwards, and should treat deliveries as at least once: deduplicate on the envelope `id`.

### Network policy

Outgoing webhooks and the outbound HTTP calls of plugins go through the same guard against server-side request forgery:

- The scheme must be `http` or `https` (only `https` when `require_https` is on).
- The host is resolved when the endpoint is saved and again on every connection. Resolved addresses that are private, loopback, link-local, multicast, unspecified or otherwise reserved are refused. An endpoint pointing to one answers `422 webhook_url_not_allowed`, and a delivery whose host later resolves to one fails.
- Redirects are never followed.
- Only the first part of the response body is stored.

To deliver to services on a private network (a staging system, an internal ERP) the operator of the installation must set the environment variable `WEBHOOKS_ALLOW_PRIVATE_NETWORKS` to a truthy value (any value other than `0`, `false`, `no` or `off`; it is off by default). This is infrastructure configuration read from `.env`, not an organization setting, and it applies to every outbound request the guard covers, so enable it only where the network itself is trusted. See [Configuration](/_aphel/docs/admin/configuration/).

## Incoming webhooks

An **incoming webhook** is a secret URL that accepts a JSON or form payload from an external system and turns it into records. Managing them needs `settings.manage`.

| Route | Purpose |
| --- | --- |
| `GET` and `POST /api/v1/incoming-webhooks` | List and create. |
| `GET /api/v1/incoming-webhooks/targets` | Record types and fields the actions can write. |
| `GET`, `PATCH`, `DELETE /api/v1/incoming-webhooks/{webhook_id}` | Read, change or delete. |
| `POST .../regenerate-token` | Issue a new URL token; the old one stops working. |
| `POST` and `DELETE .../signing-secret` | Create or remove the signing secret. |
| `GET .../requests` and `GET .../requests/{request_id}` | The log of received requests. |
| `POST /api/v1/hooks/{token}` | The public receiver. |

### Creating one

```bash
curl -X POST "$APHEL_URL/api/v1/incoming-webhooks" \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Website form",
    "generate_signing_secret": true,
    "signature_tolerance_seconds": 300,
    "actions": [
      {
        "type": "upsert_record",
        "record_type": "people.person",
        "match_field": "email",
        "fields": {
          "first_name": "$.contact.first_name",
          "last_name": "$.contact.last_name",
          "email": "$.contact.email",
          "source": "website"
        }
      }
    ]
  }'
```

The `201` response includes `token` (it starts with `whk_`), `path` (`/api/v1/hooks/<token>`) and, with `generate_signing_secret`, `signing_secret` (`whsec_...`). All three are shown only once; later reads show `token_prefix` and `secret_configured`.

Actions run in order, as the member who created the webhook, with their permissions at the time of each request. An installation can have at most 50 incoming webhooks (`422 too_many_incoming_webhooks`), and a webhook has up to 10 actions.

| Action property | Meaning |
| --- | --- |
| `type` | `create_record` or `upsert_record`. |
| `record_type` | The record type to write, such as `people.person`. |
| `match_field` | For `upsert_record`: the field used to find an existing record to update. |
| `fields` | Map of field key to a value expression (up to 60 fields). |
| `assignment_rule_id` | Optional lead distribution rule used to pick the owner of created records. |

A value expression in `fields` is one of:

| Expression | Example | Result |
| --- | --- | --- |
| JSON path | `"$.contact.email"`, `"$.items[0].sku"`, `"$['full name']"` | The value found in the payload. A missing path leaves the field unset. |
| Template | `"{{$.first}} {{$.last}}"` | Text with paths replaced by their scalar value. |
| Constant | `"website"`, `42`, `true` | The value as is. |

Paths start with `$`, have at most 200 characters and use `.key`, `["key"]` or `[index]` segments.

### Sending a request

The receiver needs no session, API token or `X-Requested-With` header: the token in the URL authenticates the request. It accepts `application/json` (an object or an array) and `application/x-www-form-urlencoded` (turned into an object; repeated keys become arrays), up to 256 KB.

```bash
curl -X POST "$APHEL_URL/api/v1/hooks/$WEBHOOK_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"contact":{"first_name":"Ana","last_name":"Souza","email":"ana@acme.com.br"}}'
```

A webhook with a signing secret requires `X-Aphel-Signature: t=<unix seconds>,v1=<hex>`, built exactly as for outgoing deliveries (HMAC-SHA256 of `<t>.<raw body>` keyed with the signing secret). `t` must be within the tolerance of the webhook (300 seconds by default, between 30 and 86400). Send `X-Aphel-Delivery` with a unique id per delivery; a signed request is identified by its signature. A repetition within 24 hours answers `409 webhook_replayed`.

Success answers `200` with `{ "data": { "request_id": "...", "status": "succeeded" } }`. Every request is logged and visible in `.../requests`; on failures the same id is in `error.details.request_id`.

| Status | Code | Cause |
| --- | --- | --- |
| 401 | `invalid_webhook_signature` | Missing, malformed or wrong signature, or a timestamp outside the window. |
| 403 | `incoming_webhook_disabled`, `incoming_webhook_owner_unavailable`, `incoming_webhook_permission_denied` | Disabled, its creator is no longer an active member, or the creator lacks a permission an action needs. |
| 404 | `incoming_webhook_not_found` | Unknown token. |
| 409 | `webhook_replayed` | Delivery already received in the last 24 hours. |
| 413 | `webhook_payload_too_large` | Body over 256 KB. |
| 422 | `invalid_webhook_payload`, `webhook_mapping_failed` | Unsupported content type or malformed body, or an action rejected the mapped values (`details.fields` use `actions.<i>.<field>`). Earlier actions are kept and the log shows a partial result. |
| 429 | `rate_limited` | More than 120 requests in the last minute for this URL. These requests are not logged. |

## Other callbacks

Other modules receive provider callbacks on their own public routes under `/api/v1/public/` (WhatsApp Cloud API, the electronic signature provider, sign-in providers). They are tied to the module that configures them and verify the provider's own signature; see [Authentication](/_aphel/docs/reference/authentication/#public-endpoints).
