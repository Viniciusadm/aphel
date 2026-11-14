---
title: REST API
description: Base path, envelopes, headers, versioning, safe writes, the OpenAPI document and a map of the route groups of the Aphel API.
sidebar:
  order: 1
---

Everything the Aphel interface does goes through the REST API, and the same API is available to integrations. Every route is annotated and appears in the generated OpenAPI document, which is the exact contract; this page explains the conventions that apply to all routes.

## Base path, documentation and versioning

| What | Where |
| --- | --- |
| API base path | `/api/v1` |
| OpenAPI 3 document (JSON) | `/api/openapi.json` |
| Interactive documentation (Scalar) | `/api/docs` |
| Print the document from the binary | `aphel openapi` |
| Health endpoints | `/_aphel/health`, `/_aphel/health/ready` and `/_aphel/metrics`, outside `/api/v1` |

The OpenAPI document is generated from the modules compiled into the binary. It lists every route of those modules, whether or not a module is currently enabled; a request to the route of a disabled module answers `404 module_disabled`. Each tag of the document carries an `x-aphel-module` extension with the module that owns it.

The version is part of the path. Changes that break the contract of `/api/v1` ship only under a new `/api/v2` prefix; additions (new routes, new optional fields, new response properties, new error codes) can appear in `/api/v1` at any time, so clients must ignore unknown properties.

## Request format

- Bodies of `POST`, `PUT` and `PATCH` requests are JSON and must be sent with `Content-Type: application/json` (or an `application/*+json` type). Any other content type answers `422 validation_error`.
- Uploads use `multipart/form-data` (see [Records API](/_aphel/docs/reference/records-api/#file-attachments)). The public incoming-webhook receiver also accepts `application/x-www-form-urlencoded` (see [Webhooks and events](/_aphel/docs/reference/webhooks-and-events/#incoming-webhooks)).
- Body fields that a route does not know are rejected on routes that declare strict bodies (`422 validation_error` with the field in `details.fields`); do not rely on extra fields being ignored.
- Identifiers are UUIDs (version 7) as strings.
- Timestamps are ISO-8601 in UTC, such as `2026-10-20T13:45:00Z`. Some record `updated_at` values keep sub-second precision, such as `2026-10-20T13:45:00.123456Z`.
- Money is a decimal string with a currency code, such as `"amount": "15000.00", "currency": "BRL"`.
- Custom field values are returned in the `custom_fields` object of the record, keyed by field key.
- Cross-origin requests are accepted (the server answers with a permissive CORS policy), but a browser session cookie is still subject to its `SameSite=Lax` attribute. Integrations should use [API tokens](/_aphel/docs/reference/authentication/#api-tokens).

## Response envelopes

| Shape | Used for | Example |
| --- | --- | --- |
| `{ "data": { ... } }` | One item | `{ "data": { "id": "0192a1b2-0000-7000-8000-000000000001", "name": "Acme" } }` |
| `{ "data": [ ... ], "meta": { "total", "page", "per_page" } }` | Paginated lists | see [Pagination and filters](/_aphel/docs/reference/pagination-and-filters/) |
| `{ "data": [ ... ], "meta": { "next_cursor" } }` | Cursor lists, such as the record timeline | `{ "data": [], "meta": { "next_cursor": null } }` |
| `{ "error": { "code", "message", "details" } }` | Every error | see [Errors](/_aphel/docs/reference/errors/) |
| empty body | `204 No Content` actions, such as logout or delete | |

Status codes follow the usual meaning: `200` for reads and updates, `201` for creations, `202` for work accepted into the background queue, `204` for actions with no result. Errors always use the envelope, including the ones raised before a handler runs, such as an oversized body (`413 payload_too_large`).

## Headers

### Sent by the client

| Header | Required | Purpose |
| --- | --- | --- |
| `Authorization: Bearer aphel_<prefix>_<secret>` | For API tokens | Authenticates the request with an API token. |
| `Cookie: aphel_session=...` | For browser sessions | Set by `POST /api/v1/auth/login`. |
| `X-Requested-With: aphel` | For session requests that change data | CSRF protection. Not needed with a Bearer token or on `/api/v1/public/*` and `/api/v1/hooks/*`. See [Authentication](/_aphel/docs/reference/authentication/#csrf-header). |
| `Content-Type: application/json` | On requests with a JSON body | |
| `Idempotency-Key: <string>` | Optional on `POST` | Makes a creation safe to repeat. |
| `If-Match: <updated_at>` | Optional on `PATCH` | Optimistic concurrency. |

### Sent by the server

| Header | When | Meaning |
| --- | --- | --- |
| `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset` | Every response to a request that used an API token | Size of the window, requests left and seconds until the window frees up. |
| `Retry-After` | `429` responses | Seconds to wait. |
| `Idempotent-Replayed: true` | Replayed idempotent responses | The response was stored from the first request. |
| `Set-Cookie: aphel_session=...` | Login, two-factor challenge, logout | Starts or clears the session cookie. |
| `Cache-Control: no-store` | `/_aphel/health/ready` and `/_aphel/metrics` | |
| `X-Content-Type-Options: nosniff` | File downloads | |

The API does not send `ETag` headers. The version of a record is its `updated_at` value.

## Safe writes

### Partial updates and optimistic concurrency

`PATCH` is a partial update: properties you omit stay unchanged and `null` clears an optional one. To avoid overwriting someone else's change, send the `updated_at` you read in `If-Match` (quotes and a leading `W/` are tolerated):

```bash
curl -X PATCH "$APHEL_URL/api/v1/people/$PERSON_ID" \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -H "Content-Type: application/json" \
  -H "If-Match: 2026-10-20T13:45:00.123456Z" \
  -d '{"job_title":"Head of Sales"}'
```

If the record changed since that version the answer is `409 version_conflict`; read the record again and re-apply the change. A value that is not an ISO-8601 timestamp answers `422 validation_error` on the field `If-Match`. The check is repeated inside the write transaction, so two concurrent updates cannot both succeed. Timestamps without fractional seconds are compared at one-second precision, those with milliseconds at millisecond precision, and the others at microsecond precision.

### Idempotency

Send `Idempotency-Key` on a `POST` to repeat it safely:

- The key has at most 255 printable ASCII characters. A longer or invalid key answers `422 validation_error`.
- The same key with the same method, path (including the query string) and body, used with the same credential (the session or the API token), returns the stored response with the same status and body plus `Idempotent-Replayed: true`. Keys are kept for 24 hours.
- The same key with a different request answers `422 idempotency_key_reused`.
- A second request while the first is still running answers `409 idempotency_key_in_progress`; retry after a short wait. An in-progress marker that is never completed expires after 10 minutes.
- `5xx` and `429` responses, and responses larger than 1 MiB, are not stored, so the request can be retried with the same key.
- The key is ignored on `/api/v1/auth/*`, `/api/v1/installer*` and `/api/v1/hooks/*`, on methods other than `POST` and on requests without a valid credential.

```bash
curl -X POST "$APHEL_URL/api/v1/organizations" \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: 6f1d2c1e-3b4a-4c55-9a77-0d9e8f7a6b5c" \
  -d '{"name":"Acme Ltda","custom_fields":{}}'
```

## Rate limits

Requests made with an API token are limited per token with a sliding window; the default is 600 requests per minute, configurable by an administrator between 1 and 100000 requests and between 1 and 3600 seconds (`api_tokens.limit` and `api_tokens.window_secs` in the organization settings). Session requests are not counted by this limiter, but sign-in, password reset and similar public actions have their own limits, described in [Authentication](/_aphel/docs/reference/authentication/#rate-limits-and-lockout).

## Health and metrics

| Route | Auth | Answer |
| --- | --- | --- |
| `GET /_aphel/health` | none | `200` with the text `ok` while the process is up. |
| `GET /_aphel/health/ready` | none | `200` when every check passes, `503` otherwise. The JSON body has `status`, `checked_at` and `checks[]` (`name`, `status`, and when relevant `reason`, `age_seconds`, `threshold_seconds`). Checks cover the database, storage, worker heartbeat, scheduler and queue delay. |
| `GET /_aphel/metrics` | `Authorization: Bearer <METRICS_TOKEN>` | Prometheus text format (version 0.0.4) with build information, database availability, job counts, the age of the oldest ready job, scheduler delay, live workers and HTTP responses by status class. `404 metrics_disabled` when `METRICS_TOKEN` is not set and `401 metrics_unauthorized` for a wrong token. |

These routes are mounted at the server root, not under `/api/v1`. See [Operations](/_aphel/docs/admin/operations/) for how to use them in monitoring.

## Route groups

The table lists every tag of the OpenAPI document, the module that contributes it and the number of operations it holds in the document generated from this build. Routes of a tag exist only while its module is enabled. Per-record routes, such as `/people/{id}`, follow the conventions in [Records API](/_aphel/docs/reference/records-api/).

| Tag | Module | Operations | Purpose |
| --- | --- | --- | --- |
| `aggregates` | `kernel` | 2 | Aggregation engine: grouped counts, sums and averages of records with periods, comparison with the previous period and filters on related records |
| `api-tokens` | `kernel` | 4 | Personal API tokens with scopes for integrations (session only) |
| `audit` | `kernel` | 1 | Audit log of the active organization |
| `auth` | `kernel` | 11 | Authentication, sessions and invitations |
| `directory` | `kernel` | 1 | Directory of the active organization's users, readable by every member |
| `field-permissions` | `kernel` | 2 | Field permissions per role: hidden, read only or editable |
| `files` | `kernel` | 5 | File uploads, downloads and record attachments |
| `health` | `kernel` | 3 | Service health |
| `installer` | `kernel` | 6 | First-run installation wizard |
| `members` | `kernel` | 10 | Members and invitations of the active organization |
| `modules` | `kernel` | 7 | Modules compiled into this build: manifest, compatibility, enabling, settings and data removal |
| `objects` | `kernel` | 9 | Dynamic entity records |
| `organization` | `kernel` | 6 | Active organization settings |
| `preferences` | `kernel` | 2 | Personal preferences of the current user, such as keyboard shortcuts |
| `record-sharing` | `kernel` | 4 | Sharing of any record with users or teams (read or edit access) on top of the role scopes |
| `roles` | `kernel` | 6 | Roles, permission matrix and permission catalog |
| `security` | `kernel` | 6 | Account security: active sessions, personal API tokens, password change and the security event log |
| `support-access` | `kernel` | 0 | Temporary support access the organization grants to installation administrators |
| `system` | `kernel` | 8 | System administration: background jobs |
| `teams` | `kernel` | 5 | Teams, managers and team hierarchy |
| `two-factor` | `kernel` | 8 | Two-factor authentication (TOTP): setup, confirmation, recovery codes, login challenge and organization policy |
| `ai` | `ai` | 6 | AI assistant: record summaries, next actions, writing help and natural language queries |
| `approvals` | `approvals` | 13 | Approval policies (record type, moment, condition, sequential steps) and approval requests with decisions |
| `assignment` | `assignment` | 8 | Record assignment rules (round robin, weighted, least loaded, routing by field), their simulation and the rotation state used by automations, incoming webhooks and imports |
| `automations` | `automations` | 14 | No-code automations: triggers, conditions and actions over records, versions, dry runs and run history |
| `booking` | `booking` | 18 | Booking links: availability, meeting types, team links and the public booking page |
| `bulk` | `bulk_actions` | 5 | Bulk actions over selected records or every record of a filter, with progress and report |
| `calendar` | `calendar` | 2 | Calendar entries from enabled providers |
| `catalog-items` | `catalog_items` | 8 | Catalog of products and services |
| `config-packages` | `config_packages` | 8 | Configuration packages |
| `dashboard` | `dashboard` | 6 | Dashboard widgets, their data and the per-user layout |
| `deal-members` | `deals` | 9 | Team of a deal (members, roles and revenue credit split) and the configurable member roles |
| `deals` | `deals` | 13 | Deals, stage changes and the Kanban board |
| `deals-reports` | `deals` | 2 | Deal conversion and sales performance reports |
| `demo` | `demo` | 1 | Demonstration environment status |
| `duplicates` | `duplicates` | 9 | Duplicate detection rules, the duplicate queue and record merging |
| `email settings` | `email` | 5 | Organization e-mail settings: relationship capture and blocklist |
| `email templates` | `email` | 6 | Organization and personal e-mail templates with record variables |
| `email threads` | `email` | 7 | E-mail conversations linked to records, visibility, manual links and attachments |
| `email tracking` | `email` | 7 | Open and click tracking settings, objections and the public tracking endpoints |
| `emails` | `email` | 3 | Sending e-mail from records through the user's mailbox, booking, cancellation and rendered previews |
| `mailboxes` | `email` | 8 | Personal mailboxes (IMAP/SMTP and module providers), connection test and synchronization |
| `enrichment` | `enrichment` | 3 | Company enrichment by CNPJ from public sources |
| `esign` | `esign` | 6 | Sends PDFs attached to deals for electronic signature on Clicksign and tracks their status through webhooks |
| `exports` | `exports` | 3 | CSV and XLSX exports of filtered records |
| `favorites` | `favorites` | 4 | Personal favorites: pinned records and saved views shown in the sidebar |
| `followers` | `followers` | 4 | Following registered records and owner-provided events |
| `forecast` | `forecast` | 4 | Revenue forecast: organization settings and the forecast per month |
| `forms` | `forms` | 12 | Public forms that create people and deals |
| `fields` | `formulas` | 18 | Formulas |
| `goals` | `goals` | 6 | Sales goals per user, team or organization and their progress |
| `google` | `google` | 7 | Login with Google, Google Calendar meeting sync and Gmail mailboxes |
| `imports` | `imports` | 8 | CSV and XLSX imports: upload, mapping, dry run, execution and retry of failed rows |
| `integrations` | `integrations` | 7 | Integration center: providers, configuration, connection test and activation |
| `lifecycle` | `leads` | 12 | Lifecycle stages of people and organizations: automatic transitions, stage history, conversion report, lead inbox actions (qualify, disqualify, convert) and disqualification reasons |
| `line-items` | `line_items` | 8 | Scoped record line items and total policy |
| `catalog` | `marketplace` | 4 | Plugin and configuration marketplace |
| `communication` | `messaging` | 3 | Communication opt-outs |
| `messaging` | `messaging` | 16 | Shared inbox for WhatsApp Cloud API and email (IMAP/SMTP) |
| `microsoft` | `microsoft` | 7 | Login with a Microsoft account (Entra ID), Outlook calendar meeting sync and Outlook mailboxes |
| `migration-assistant` | `migrations_assistant` | 4 | Assisted migration from other CRMs (Pipedrive, HubSpot, RD Station CRM, Agendor): recognized columns, conversion of their CSV exports and pre-filled imports |
| `currencies` | `money` | 9 | Enabled currencies of the organization and dated exchange rates to the base currency |
| `notes` | `notes` | 7 | Notes attached to records |
| `notifications` | `notifications` | 9 | Notifications and delivery preferences |
| `custom-objects` | `objects` | 6 | Object administration |
| `relationships` | `objects` | 8 | Object relationships |
| `organization-exports` | `org_export` | 4 | Full export of the organization (zip with records, relationships, files, configuration and manifest) for backup and restoration with `aphel org import` |
| `organizations` | `organizations` | 9 | Companies |
| `people` | `people` | 11 | Contacts and their organizations |
| `pricing` | `pricing` | 9 | Price lists by currency, segment or company with validity periods and quantity tiers, and the maximum discount per role |
| `privacy` | `privacy` | 17 | Privacy (LGPD): personal data subjects, data subject requests (export, anonymization, deletion) and retention policies |
| `proposals` | `proposals` | 10 | Proposal templates and PDF proposals generated from deals |
| `quotes` | `quotes` | 18 | Quotes of deals: versions, items, discounts and taxes, approval, sending, the signed public page and acceptance |
| `layouts` | `record_layouts` | 3 | RecordLayouts |
| `reminders` | `reminders` | 5 | Reminders: before a task is due and standalone reminders linked to a record |
| `reports` | `reports` | 13 | Saved custom reports, their results, exports and e-mail schedules |
| `views` | `saved_views` | 6 | SavedViews |
| `scoring` | `scoring` | 7 | Lead scoring models (profile and behavior criteria, decay, bands) and the scores of a record with their explanation |
| `search` | `search` | 1 | Search |
| `segments` | `segments` | 6 | Segments: named dynamic filters with member count, usable as the `in_saved_filter` filter condition |
| `sequences` | `sequences` | 12 | Sales sequences (cadences): steps, enrollments, automatic exits, sending limits and per-step metrics |
| `sso` | `sso` | 3 | Single sign-on with any corporate OpenID Connect provider |
| `tagging` | `tagging` | 9 | Tags of the organization and tagging of opted-in records |
| `task_types` | `tasks` | 7 | Activity types of the organization |
| `tasks` | `tasks` | 8 | Tasks and logged tasks |
| `telephony` | `telephony` | 19 | Call logging as tasks, call outcomes, call sessions over saved views and the calls report |
| `themes` | `themes` | 9 | Installable visual themes: design tokens and fonts applied to the whole installation |
| `timeline` | `timeline` | 2 | Unified timeline of a record |
| `trash` | `trash` | 6 | Deleted records: restore, permanent deletion and retention |
| `validation-rules` | `validation_rules` | 7 | ValidationRules |
| `plugins` | `wasm_runtime` | 13 | Runtime plugins: install, enable, disable, configure and uninstall .aphelplugin packages, and call their routes |
| `events` | `webhooks` | 1 | Catalog of public event types delivered by outgoing webhooks |
| `incoming-webhooks` | `webhooks` | 12 | Incoming webhooks: secret URLs that create or update records from external systems |
| `webhooks` | `webhooks` | 10 | Outgoing webhooks: endpoints, signed deliveries, test events and redelivery |
| `workflow` | `workflow` | 28 | Pipelines, stages and lost reasons |

## Which credentials a route accepts

Routes tagged `auth`, `api-tokens`, `installer` and `two-factor` accept only the browser session; calling them with a Bearer token answers `403 api_token_not_allowed`. The other protected routes accept either a session or an API token, and the effective permissions of a token are limited by its scopes. Public routes (`/api/v1/public/*`, `/api/v1/hooks/*` and the first-run installer) need no credential. Details in [Authentication](/_aphel/docs/reference/authentication/).
