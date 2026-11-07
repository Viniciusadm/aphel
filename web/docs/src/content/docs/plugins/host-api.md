---
title: Host API
description: Every function a plugin backend can call, with arguments, JSON shapes, limits, error codes and the contexts where each is allowed.
sidebar:
  order: 12
---

The host API is the only way a backend reaches Aphel. It is the `host` interface of the WebAssembly contract `aphel:plugin@1.0.0`. The [JavaScript SDK](/docs/plugins/js-sdk/) and the [Rust SDK](/docs/plugins/rust-sdk/) wrap it; this page describes the functions at the contract level and gives the SDK names.

## Conventions

- Complex values travel as **JSON strings**. The SDKs parse and stringify for you. Record ids are UUID strings. Timestamps are RFC 3339.
- A function that fails returns a **plugin error**: `{ code, message, status, fields }`. The JavaScript SDK throws a `PluginError`, the Rust SDK returns `Err(PluginError)`. Host errors keep the HTTP status and code that the REST API would use, so a plugin that does not catch them propagates them to the caller. Internal failures (HTTP 500) are reported with the code `internal_error` and the message `internal error`. A database deadlock or lock timeout is reported as `409 version_conflict`.
- Every record function uses the permissions of the current call: the actor's permissions intersected with the plugin's capabilities. See [Permissions and roles](/docs/plugins/permissions-and-roles/).

### Where side effects are allowed

The three restrictions below apply to the functions marked in the **Effect** column of this page. Reads are allowed everywhere.

| Entry point | Direct effects | `defer-effect` | Error when refused |
| --- | --- | --- | --- |
| Record hooks and domain hooks, including `record.committed` | No | Yes (stored with the transaction) | `409 plugin_write_in_hook` |
| Hook re-entered by the same plugin's own write | No | No | `403 plugin_read_only` |
| Event, schedule, automation action, privacy request | No | Yes | `409 plugin_effect_required` |
| Private or public route | Yes | Yes | |
| Operation provided by the plugin, effect `write` or `external`; contribution | Yes | Yes | |
| Operation provided by the plugin, effect `read`; dashboard widget | No | No | `403 plugin_read_only` |

The functions guarded by this table are `create-record`, `update-record`, `delete-record`, `update-record-if`, `apply-records`, `upload-file`, `kv-set`, `kv-delete`, `kv-increment`, `kv-set-if-absent`, `next-number`, `http-fetch`, `notify` and `emit`. `invoke` of a writing operation follows the same table.

## Records

Record type keys are strings such as `library.book` or `people.person`. Records are JSON objects as described in [Entities and fields](/docs/plugins/entities-and-fields/#record-shape).

### get-record

`get-record(record-type, id) -> string?` (JavaScript `ctx.records.get`, Rust `ctx.records().get`)

Returns the record as a JSON string, or nothing when it does not exist or the caller cannot see it. Needs the read permission of the type (otherwise `403`).

### list-records

`list-records(record-type, query) -> { records-json, total }` (JavaScript `ctx.records.list` returns `{ data, total }`)

| Query field | Type | Description |
| --- | --- | --- |
| `filter-json` | string, optional | A filter, see below. |
| `sort` | string, optional | Comma-separated fields, `-` prefix for descending: `-created_at,title`. Up to 3. |
| `search` | string, optional | Text search, up to 100 characters. |
| `page` | integer, optional | Starts at 1. Default 1. |
| `per-page` | integer, optional | 1 to 100. Default 25. |

Archived records are excluded. A **filter** is a condition or a group of conditions.

```json
{
  "op": "and",
  "conditions": [
    { "field": "status", "operator": "eq", "value": "reading" },
    { "field": "pages", "operator": "gte", "value": 300 }
  ]
}
```

A condition is `{ "field", "operator", "value" }`; a group is `{ "op": "and" | "or", "conditions": [...] }`; a related filter is `{ "related": { "relationship", "filter", "none" } }`. Operators: `eq`, `neq`, `contains`, `not_contains`, `starts_with`, `gt`, `gte`, `lt`, `lte`, `between`, `in`, `not_in`, `is_empty`, `is_not_empty`, `today`, `yesterday`, `tomorrow`, `this_week`, `last_week`, `next_week`, `this_month`, `last_month`, `next_month`, `this_year`, `within_last_days`, `within_next_days`, `older_than_days`, `anniversary`, `in_saved_filter`, `in_group`. Limits: depth 4, 50 conditions, 100 values in a list. A filter that does not parse fails with `422`: `the filter must be a filter group or condition`. Filtering or sorting by a field the actor cannot see fails with `422 field_not_accessible`.

### aggregate-records

`aggregate-records(query-json) -> string` (JavaScript `ctx.records.aggregate`)

Runs an aggregation with the caller's read scope.

```json
{
  "record_type": "library.book",
  "group_by": [{ "field": "status" }],
  "metrics": [{ "fn": "count" }, { "fn": "sum", "field": "pages" }]
}
```

Keys: `record_type` (required), `filter`, `group_by` (list of `{ field, granularity? }`), `metrics` (list of `{ fn, field?, to?, unit? }` with `fn` one of `count`, `sum`, `avg`, `min`, `max`), `compare_to_previous`, `period` (`{ field, preset?, from?, to? }`), `sort`, `limit` and `archived` (`exclude`, `include` or `only`). Unknown keys fail with `422`. The result has the same shape as the aggregate endpoint described in [Records API](/docs/reference/records-api/).

### iterate-records

`iterate-records(record-type, query, cursor) -> (records, next-cursor)` (JavaScript `ctx.records.iterate` returns `{ data, cursor }`)

Walks all records that match a filter in pages. `query` is the same as for `list-records`; `per-page` defaults to 100 (1 to 100) and `sort` defaults to `created_at`. Pass no cursor for the first page, then the returned cursor until it is empty. A cursor from elsewhere fails with `422`: `the cursor is invalid`. The cursor is an offset, so avoid changing the matching set while iterating.

### create-record, update-record, delete-record

| Function | Returns | JavaScript | Rust |
| --- | --- | --- | --- |
| `create-record(record-type, values-json)` | the new record | `ctx.records.create` | `ctx.records().create` |
| `update-record(record-type, id, values-json)` | the updated record | `ctx.records.update` | `ctx.records().update` |
| `delete-record(record-type, id)` | nothing | `ctx.records.delete` | `ctx.records().delete` |

Effect: yes. `values-json` is an object of the fields to set. Writes go through the full record pipeline: authorization of the create/update/delete permission, hooks (including the plugin's own, which then run read-only), validation, audit and events. A record type that does not support generic writes fails with `422 record_operation_not_supported`. An invalid id fails with `422`: `the record id is invalid`.

### update-record-if

`update-record-if(record-type, id, expected-updated-at, values-json) -> string`

Like `update-record`, but locks the row and fails with `409 version_conflict` unless the stored `updated_at` equals `expected-updated-at` (RFC 3339). Use it for read-modify-write cycles. Effect: yes.

### apply-records

`apply-records(mutations) -> list of records` (JavaScript `ctx.records.apply`, Rust `ctx.records().apply`)

Applies several creates and updates **in one database transaction**. Each mutation is `{ record-type, id?, expected-updated-at?, values-json }`: no `id` means create; with `id`, update; `expected-updated-at` only on updates and checked like `update-record-if`.

- 1 to 100 mutations and at most 1 MiB of `values-json` in total, otherwise `422 plugin_batch_invalid`.
- A value of the form `{ "$ref": n }` anywhere in the values is replaced by the id of the record produced by mutation number `n` (starting at 0, earlier in the same batch). A reference to a later or missing mutation fails the batch.
- Any failure rolls back the whole batch. Errors are prefixed with `mutations[<index>]` and, for validation errors, the field paths too. Code `plugin_batch_invalid` is used for validation failures of the batch.
- The result lists the final records in the order of the mutations. Commit-time hooks (`record.committed`) run after the batch commits.

```js
ctx.records.apply([
  { recordType: 'library.shelf', values: { name: 'Fiction' } },
  { recordType: 'library.book', values: { title: 'Dune', shelf: ctx.records.ref(0) } },
])
```

Effect: yes.

### set-audit-reason

`set-audit-reason(reason?)` (JavaScript `ctx.setAuditReason`)

Sets a free-text reason (trimmed; empty clears it) that is stored with the audit entries of the record writes made afterwards in the same call. Not guarded.

## Files

| Function | Description |
| --- | --- |
| `upload-file(record-type, record-id, filename, body) -> string` | Stores a file attached to a record. Returns the file as a JSON string: `{ id, name, content_type, size, checksum, created_at, download_path }`. The upload limits and permissions of the files API apply. Effect: yes. JavaScript `ctx.uploadFile`, Rust `ctx.upload_file`. |
| `read-file(file-id) -> (file-json, bytes)` | Reads a file the caller may download. Fails with `413 plugin_file_too_large` above 10 MiB. JavaScript `ctx.readFile` returns `{ file, body }`. |
| `respond-bytes(content-type, filename?, body)` | Makes the route answer with a file. Routes only. See [Routes](/docs/plugins/routes/#downloads-and-raw-bodies). |

## Authorization

| Function | Returns | JavaScript | Rust |
| --- | --- | --- | --- |
| `can(permission)` | boolean: whether the current call holds the permission (any scope) | `ctx.can` | `ctx.can` |
| `permissions()` | the permission keys the current call holds | `ctx.permissions` | `ctx.permissions` |

Both reflect the actor ∩ capabilities of the current call, also inside hooks.

## Key-value storage

Each plugin has private storage: collections of items identified by an id, holding any JSON value. It is separate from records, not searchable, kept across upgrades and deleted on uninstall.

| Function | Description |
| --- | --- |
| `kv-get(collection, id) -> string?` | The stored JSON, or nothing. |
| `kv-set(collection, id, value-json)` | Creates or replaces the item. Effect: yes. |
| `kv-delete(collection, id)` | Deletes the item. Effect: yes. |
| `kv-list(collection) -> list of (id, value-json)` | Up to 500 items ordered by id. |
| `kv-increment(collection, id, delta) -> integer` | Atomically adds `delta` to an integer item (a missing item starts at 0) and returns the new value. Fails with `409 plugin_kv_not_a_counter` if the stored value is not an integer, and `422` on overflow. Effect: yes. |
| `kv-set-if-absent(collection, id, value-json) -> boolean` | Creates the item only if it does not exist. Returns whether it was created. Use it for locks and one-time markers. Effect: yes. |
| `next-number(sequence) -> integer` | Atomically returns the next number of a named sequence, starting at 1. Stored in the collection `_sequences`. Fails with `409 plugin_kv_not_a_counter` if the sequence was set to a negative value. Effect: yes. |

Collection names have 1 to 64 characters and ids 1 to 128, otherwise `422`. In JavaScript use `ctx.kv('name')` and its methods `get`, `set`, `delete`, `list`, `increment`, `setIfAbsent`; in Rust `ctx.kv("name")` with the same methods in snake case. `ctx.nextNumber` / `ctx.next_number` for sequences.

## HTTP

The HTTP request WIT contract now includes `client-identity`. Rebuild plugin components with the updated SDK before loading them in this runtime; there is no compatibility adapter for the previous contract.

`http-fetch(request) -> response` (JavaScript `ctx.http.send` and `ctx.http.get`, Rust `ctx.http().send` and `.get`). Effect: yes (so only in routes, and operations with a write or external effect; elsewhere use an `http` deferred effect).

| Request field | Description |
| --- | --- |
| `method` | `GET`, `POST`, ... (case-insensitive). |
| `url` | Absolute URL. |
| `headers` | List of name/value pairs. |
| `body` | Optional bytes, up to 5 MiB. |
| `client-identity` | Optional name of a declared `tls_identity` setting; JS `clientIdentity`, Rust `.client_identity(name)`. See [Client certificates](/docs/plugins/settings-and-secrets/#client-certificates-mtls). |

The response has `status`, `headers` and `body` (bytes, up to 5 MiB). A non-2xx status is returned as a normal response, not as an error. The JavaScript response also has `ok`, `text()`, `json()` and `header(name)`; the request accepts `json` as a shortcut for a JSON body.

Policy:

- The URL host must match `[http] allowed_hosts`, otherwise `403 plugin_host_not_allowed`.
- Only `http` and `https` URLs without credentials or fragments. HTTPS may be required by the installation's outbound policy. Private networks, loopback and local names (`localhost`, `.local`, `.internal`) are refused unless the operator allowed them. A refused URL gives `422 plugin_host_not_allowed`.
- Redirects are not followed: a `3xx` is returned to you. No proxy is used.
- The call is limited by the time left in the current entry point (a request needs at least 100 ms left) and 5 seconds to connect. Failures give `503 plugin_http_failed`; so does a response above 5 MiB.
- The headers `host`, `content-length`, `connection`, `transfer-encoding`, `proxy-authorization` and `upgrade` are dropped.

## Notifications and events

| Function | Description |
| --- | --- |
| `notify(notification)` | Sends an in-app/email/push notification of a declared kind to a user. Effect: yes. Fields and errors in [Automation and widgets](/docs/plugins/automation-and-widgets/#notifications). JavaScript `ctx.notify({ kind, recipientUserId, title, body, record, link, data, dedupeKey })`. |
| `emit(name, record-type?, record-id?, payload-json)` | Publishes `plugin.<key>.<name>`. The name must be declared in `[[emits]]` (`404 plugin_event_not_declared`), the payload a JSON object of at most 64 KiB, and the record, if given, readable. Writes an audit entry. Effect: yes. JavaScript `ctx.emit(name, { type, id } | null, payload)`. |

## Module operations

`invoke(operation, input-json) -> string` (JavaScript `ctx.invoke`, Rust `ctx.invoke`)

Calls an operation offered by a built-in module or another plugin. List the operation in the manifest's `operations` first; otherwise the call fails with `403 plugin_operation_not_declared`. The input is a JSON object (empty means `{}`), the answer a JSON value.

| Error | Meaning |
| --- | --- |
| `404 plugin_operation_unknown` | The operation does not exist or its module is disabled. |
| `409 plugin_write_in_hook` | A writing operation was called inside a hook or another in-transaction context. |
| `409 plugin_effect_required` | A writing operation was called in a retryable handler: defer it with `kind: "invoke"`. |
| `503 plugin_operation_timeout` | The operation did not finish within the time left in the call. |
| `403 plugin_query_authority_changed`, `409 plugin_query_outcome_unknown` | Replay protection of external queries, below. |

Operations marked *external query* (`ai.*`, `enrichment.lookup`) reach outside services. Their result is saved with the execution (keyed by operation, input and authority), so a retry of the same execution reuses the answer instead of calling the service again. If an earlier attempt started but did not store a result, the retry fails with `409 plugin_query_outcome_unknown` and the call is not repeated.

The live list, including whether each module is enabled, is `GET /api/v1/wasm-runtime/host-catalog`. The SDKs ship constants for them (`operations` in both). Operations marked **durable** can be deferred with `kind: "invoke"`: they write everything in one transaction without external calls, so a retry never repeats them. Each operation needs the permissions of the module's API, so declare them in `requires`.

| Operation | Access | Durable | Input and result |
| --- | --- | --- | --- |
| `ai.complete` | write | no | `{ system?, prompt }` or `{ system?, messages: [{ role, content }] }`, `max_tokens?` (1 to 4096), `temperature?` (0 to 1) returns `{ text, input_tokens, output_tokens, provider, model }` |
| `ai.compose` | write | no | `{ context: note\|email\|message, instruction, draft?, record_type?, record_id? }` returns `{ text, sources, provider, model }` |
| `ai.next_actions` | write | no | `{ record_type: contact\|company\|deal, record_id }` returns `{ suggestions: [{ title, due_in_days, reason }], sources, provider, model }` |
| `ai.summary` | write | no | `{ record_type, record_id }` returns `{ text, sources, provider, model }` |
| `approvals.approve` | write | yes | `{ id, comment? }`, needs a user actor |
| `approvals.cancel` | write | yes | `{ id, comment? }`, needs a user actor |
| `approvals.reject` | write | yes | `{ id, comment }`, needs a user actor |
| `approvals.requests` | read | | `{ record_type, record_id, status?, page?, per_page? }` returns `{ data, total }` |
| `approvals.submit` | write | yes | `{ record_type, record_id, moment: on_demand\|stage_change\|quote_send, target_id?, comment?, policy_key? }` returns `{ outcome, requests }` |
| `assignment.assign` | write | yes | `{ record_type, record_id, rule?, idempotency_key? }` returns `{ rule_id, user_id, user_name, selection, route_index, reused, skipped }` |
| `booking.bookings` | read | | `{ status?, meeting_type_id?, upcoming?, page?, per_page? }` returns `{ data, total }` |
| `booking.cancel` | write | yes | `{ id, reason? }` |
| `currencies.convert` | read | | `{ currency }` returns `{ base, currency, enabled, rate }`, or `{ amount, from, to? }` returns `{ amount, from, to, base, rate, converted }` |
| `deals.move_pipeline` | write | yes | `{ id, pipeline_id, stage_id?, lost_reason_id?, lost_reason_note?, fields? }` |
| `deals.move_stage` | write | yes | `{ id, stage_id, lost_reason_id?, lost_reason_note?, fields? }` |
| `duplicates.check` | read | | `{ record_type, exclude_id?, name?, first_name?, last_name?, email?, phone?, mobile?, company_id?, domain?, website?, values? }` returns `[{ record_id, title, rules }]` (up to 10) |
| `duplicates.dismiss` | write | yes | `{ id }` |
| `duplicates.merge` | write | yes | `{ record_type, primary_id, duplicate_id, fields?, expected_updated_at? }` returns `{ record_type, survivor_id, merged_id, moved }` |
| `email.link` | write | yes | `{ thread_id, record_type, record_id }` |
| `email.send` | write | yes | `{ to: [{ address, name? }], cc?, bcc?, mailbox_id?, subject?, body_html?, body_text?, template_id?, record?, reply_to_message_id?, attachment_file_ids?, scheduled_at?, tracking? }` |
| `email.unlink` | write | yes | `{ thread_id, record_type, record_id }` |
| `enrichment.apply` | write | yes | `{ lookup_id, items: [data_key], only_blank? }` returns `{ company_id, source, cnpj, fields }` |
| `enrichment.lookup` | write | no | `{ company_id, cnpj? }` returns `{ id, company_id, cnpj, source, fetched_at, items }` |
| `esign.cancel` | write | yes | `{ id }` |
| `esign.send` | write | yes | `{ deal_id, file_id, signers: [{ name, email, contact_id? }], message?, quote_id? }` |
| `exports.request` | write | yes | `{ record_type, format: csv\|xlsx, filter?, sort?, search?, archived?, columns? }` returns the export request |
| `favorites.add` | write | yes | `{ favorite_type: record\|view, record_type?, record_id?, view_id? }`, needs a user actor |
| `favorites.list` | read | | `{}` returns `{ data }`, needs a user actor |
| `favorites.remove` | write | yes | `{ id }`, needs a user actor |
| `followers.follow` | write | yes | `{ record_type, record_id }`, needs a user actor |
| `followers.list` | read | | `{ record_type, record_id }` returns `{ record_type, record_id, following, total, followers }` |
| `followers.unfollow` | write | yes | `{ record_type, record_id }`, needs a user actor |
| `forecast.deals` | read | | `{ month: YYYY-MM, category?, owner_id?, team_id?, page?, per_page? }` returns `{ data, total }` |
| `forecast.summary` | read | | `{ owner_id?, team_id?, months? }` (at most 18 months) |
| `goals.get` | read | | `{ id }` |
| `goals.list` | read | | `{ owner_type?, user_id?, team_id?, metric?, period?, active_on?, current?, progress?, page?, per_page? }` returns `{ data, total }` |
| `leads.convert` | write | yes | `{ contact_id, pipeline_id, stage_id?, title?, amount?, currency? }` returns `{ contact_id, lifecycle_stage, deal_id }` |
| `leads.disqualify` | write | yes | `{ contact_id, reason_id, note? }` |
| `leads.qualify` | write | yes | `{ contact_id }` |
| `line_items.list` | read | | `{ record_type, record_id }` |
| `line_items.replace` | write | yes | `{ record_type, record_id, items: [...] }` |
| `messaging.reply` | write | yes | `{ conversation_id, body }` |
| `notes.create` | write | yes | `{ record_type, record_id, body, visibility?: team\|private, pinned? }` |
| `notes.delete` | write | yes | `{ id }` |
| `notes.get` | read | | `{ id }` |
| `notes.list` | read | | `{ record_type, record_id, page?, per_page? }` returns `{ data, total }` |
| `notes.pin` | write | yes | `{ id, pinned }` |
| `notes.update` | write | yes | `{ id, body?, visibility?, pinned? }` |
| `pricing.list` | read | | `{ currency?, active?, search?, page?, per_page? }` returns `{ data, total }` |
| `pricing.resolve` | read | | `{ product_id, price_list_id?, currency?, company_id?, quantity?, on? }` returns `{ price_list, unit_price, currency, quantity, on }` |
| `proposals.generate` | write | yes | `{ deal_id, template_id?, template_key?, quote_id? }` |
| `quotes.accept` | write | yes | `{ id, name?, email?, note? }` |
| `quotes.new_version` | write | yes | `{ id }` |
| `quotes.send` | write | yes | `{ id, to?, cc?, subject?, body_html?, mailbox_id? }` |
| `quotes.submit` | write | yes | `{ id, comment? }` |
| `records.archive` | write | yes | `{ record_type, id }`, returns `{ archived: true }`. Types without archive support fail with `422 record_operation_not_supported`. |
| `records.restore` | write | yes | `{ record_type, id }`, returns `{ restored: true }` |
| `reminders.create` | write | yes | `{ remind_at, note?, record_type?, record_id? }`, needs a user actor |
| `reminders.delete` | write | yes | `{ id }`, needs a user actor |
| `reminders.list` | read | | `{ status?, record_type?, record_id?, page?, per_page? }` returns `{ data, total }`, needs a user actor |
| `reports.list` | read | | `{ record_type?, page?, per_page? }` returns `{ data, total }` |
| `reports.run` | read | | `{ id, preset?, from?, to?, compare_to_previous? }` |
| `scoring.scores` | read | | `{ record_type, record_id }` |
| `scoring.signal` | write | yes | `{ record_type, record_id, signal, source }` returns `{ recorded }` |
| `segments.get` | read | | `{ id }` |
| `segments.list` | read | | `{ record_type?, counts?, page?, per_page? }` returns `{ data, total }` |
| `segments.members` | read | | `{ id, filter?, sort?, search?, archived?, page?, per_page? }` returns `{ data, total }` |
| `sequences.enroll` | write | yes | `{ sequence_id, contact_id, mailbox_id? }` |
| `sequences.exit` | write | yes | `{ id }` |
| `sequences.list` | read | | `{ active?, search?, page?, per_page? }` returns `{ data, total }` |
| `sequences.pause` | write | yes | `{ id }` |
| `sequences.resume` | write | yes | `{ id }` |
| `tagging.apply` | write | yes | `{ record_type, record_id, tag_id }` returns `{ changed, tags }` |
| `tagging.get` | read | | `{ id }` |
| `tagging.list` | read | | `{ record_type? }` returns `{ data }` |
| `tagging.remove` | write | yes | `{ record_type, record_id, tag_id }` returns `{ changed, tags }` |
| `tasks.complete` | write | yes | `{ id }` |
| `tasks.create` | write | no | `{ title, activity_type_id, due_at?, owner_id?, record_type?, record_id?, description?, duration_minutes? }` returns `{ id }` |
| `tasks.types` | read | | `{}` returns `{ items: [{ id, key, name }] }` |
| `telephony.log` | write | yes | `{ outcome_key, record_type?, record_id?, direction?, duration_seconds?, phone_number?, recording_url?, title?, notes?, occurred_at?, provider?, provider_call_id?, custom_fields? }` |
| `timeline.kinds` | read | | `{ record_type, record_id }` |
| `timeline.page` | read | | `{ record_type, record_id, cursor?, kinds?, per_page? }` returns `{ data, meta: { next_cursor } }` |
| `workflow.list` | read | | `{}` returns `{ data }` |

Operations provided by other plugins appear in the live catalog with their declared effect. Each operation's exact permission requirements are those of the corresponding module API.

## defer-effect

`defer-effect(key, command-json) -> string` (JavaScript `ctx.deferEffect(key, command)`, Rust `ctx.defer_effect(key, command)`)

Stores a command to run later in a durable job and returns the effect id. This is how hooks, events, schedules and automation actions cause side effects. It is allowed in every entry point except read-only ones (`403 plugin_read_only`).

| Rule | Value |
| --- | --- |
| Key | 1 to 128 bytes. Unique per execution. |
| Command | JSON, at most 64 KiB, one of the kinds below. Unknown kinds or properties fail with `422 plugin_effect_invalid`. |
| Same key, same command | Returns the existing effect id. Nothing is created twice. |
| Same key, different command | `409 plugin_effect_key_conflict`. |
| Effects per execution | At most 100 distinct (`422 plugin_effect_invalid` while deferring, `422 plugin_effect_limit` when persisting). |
| In a hook | Stored in the hook's transaction: kept if it commits, discarded if it rolls back. |
| Elsewhere | Stored immediately in its own transaction. |
| Authority | Captured when you defer, re-resolved when the job runs and limited to the plugin's capabilities, acting as the plugin. |
| Plugin disabled before it runs | The job fails with `503 plugin_disabled` and is retried by the queue. |

A command is checked when you defer it and again when it runs. A failing effect is retried by the job queue. A command that completed leaves a receipt, so a retry of the job does not run it again.

| `kind` | Properties | Notes |
| --- | --- | --- |
| `create` | `record_type`, `values` | The record type must support single-transaction durable mutations, else `422 plugin_effect_not_supported`. |
| `update` | `record_type`, `id`, `values` | Same. |
| `delete` | `record_type`, `id` | Same. |
| `archive` | `record_type`, `id` | Same. |
| `restore` | `record_type`, `id` | Same. |
| `kv_set` | `collection`, `id`, `value` | Collection up to 64, id up to 128 characters. |
| `kv_delete` | `collection`, `id` | |
| `emit` | `name`, `record_type?`, `record_id?`, `payload` | Name must be declared (`403 plugin_event_not_declared`). |
| `notify` | `name`, `recipient_user_id`, `title`, `body?`, `record_type?`, `record_id?`, `link?`, `data?`, `dedupe_key?` | Name must be declared (`403 plugin_notification_not_declared`). |
| `invoke` | `operation`, `input` | The operation must be listed in `operations` and be durable (`422 plugin_effect_not_supported`). |
| `email` | `to`, `to_name?`, `subject`, `text`, `html?`, `purpose?` | Needs `emails.send` in `requires` (`403 plugin_email_not_allowed`). `purpose` is `transactional` (default) or `marketing`. Subject one line of up to 200 characters; bodies up to 100 KiB. Recipients on the opt-out list are skipped. At most 120 e-mails per minute per plugin. Without `html`, `text` is escaped and used as the HTML body. |
| `http` | `method`, `url`, `headers?`, `body?`, `client_identity?`, `on_response?`, `context?` | The host must be in `allowed_hosts` (`403 plugin_host_not_allowed`). Sent with an `Idempotency-Key` header equal to the effect id and a 30 second limit. Every response is saved; network interruption produces an unknown outcome. See durable HTTP results below. |

The Rust SDK type `EffectCommand` covers the record, key-value, `emit`, `notify` and `invoke` kinds; for `email` and `http` send a JSON value built with `json!`.

Deferred commands are stored as JSON, so do not put secrets in them unless you accept that they are stored in clear until the effect completes.

## Execution context

| Function | Description |
| --- | --- |
| `hook-state-json() -> string` | The record snapshot of the current hook, or `null` outside hooks. See [Hooks](/docs/plugins/hooks/#payload). JavaScript `ctx.hookState()`. |
| `execution-context-json() -> string` | Identity of the current execution: `{ id, correlation_id, parent_id, event_id, executor, executor_kind, automation, initiator, trigger, chain }`. `trigger` is `hook`, `committed`, `event`, `schedule`, `automation`, `route`, `widget`, `operation`, `contribution` or `privacy`. `automation` is `{ automation_id, run_id, step_id }` inside automation actions. `chain` lists the plugins already in the call chain (at most 16). JavaScript `ctx.executionContext()`. |
| `settings-json() -> string` | Effective settings, without secrets. See [Settings and secrets](/docs/plugins/settings-and-secrets/). |
| `secret(key) -> string?` | A declared `secret` setting. `404 plugin_secret_not_declared` for other keys. |
| `organization-json() -> string` | `{ name, locale, time_zone, currency, primary_color }`. `404 organization_not_installed` before setup. |
| `list-users(query-json) -> string` | Active users: `{ data: [{ id, name, email, role: { key, name }, teams: [{ id, name }] }], total }`. Query keys: `search` (up to 100 characters, matches name or email), `role` (role key), `team_id`, `ids` (up to 100), `page`, `per_page` (1 to 100, default 50). Unknown keys fail with `422`. Fails with `403 plugin_directory_unavailable` when there is neither a signed-in actor nor the installer's identity. |
| `list-roles() -> string` | `[{ key, name, is_system }]`. |
| `list-teams() -> string` | `[{ id, name, parent_id }]`. |
| `now() -> string` | The current UTC time, RFC 3339. |
| `log(level, message)` | Writes to the server log with the plugin key. `error`, `warn` and `debug` map to those levels; anything else is `info`. The JavaScript `ctx.log` also accepts `trace`; `ctx.info(message)` is a shortcut. |

The directory functions (`list-users`, `list-roles`, `list-teams`) and `organization-json` do not check any permission: any plugin with a backend can call them.

## Error codes raised by the host API

| Code | Status | Raised when |
| --- | --- | --- |
| `plugin_write_in_hook` | 409 | Direct effect inside an in-transaction hook. |
| `plugin_effect_required` | 409 | Direct effect in a retryable handler. |
| `plugin_read_only` | 403 | Effect in a read-only execution (widgets, read operations, re-entered hooks). |
| `plugin_effect_invalid`, `plugin_effect_limit`, `plugin_effect_not_supported` | 422 | Bad deferred command, too many effects, record type or operation not durable. |
| `plugin_effect_key_conflict` | 409 | Same effect key with another command. |
| `plugin_email_not_allowed` | 403 | `email` effect without `emails.send` in `requires`. |
| `plugin_batch_invalid` | 422 | Bad `apply-records` batch. |
| `version_conflict` | 409 | `update-record-if` mismatch, or a lock conflict. |
| `record_not_found` | 404 | `update-record-if` or `emit` on a missing or unreadable record. |
| `record_operation_not_supported` | 422 | Record type without generic writes or archive. |
| `plugin_kv_not_a_counter` | 409 | Increment of a non-integer or negative sequence. |
| `plugin_host_not_allowed` | 403 / 422 | Host not in `allowed_hosts` / URL refused by policy. |
| `plugin_http_failed` | 503 | Network failure, no time left, response too large. |
| `plugin_http_effect_failed` | 503 | Deferred HTTP request answered non-2xx. |
| `plugin_event_not_declared` | 404 / 403 | `emit` of an event not in `[[emits]]` (host call / deferred). |
| `plugin_notification_not_declared`, `plugin_notification_invalid`, `plugin_notification_recipient_invalid`, `plugin_notification_record_not_readable` | 404 / 422 / 422 / 403 | `notify` problems. |
| `plugin_operation_not_declared`, `plugin_operation_unknown`, `plugin_operation_timeout` | 403 / 404 / 503 | `invoke` problems. |
| `plugin_secret_not_declared` | 404 | `secret` of an undeclared key. |
| `plugin_file_too_large` | 413 | `read-file` above 10 MiB. |
| `plugin_raw_body_outside_route`, `plugin_raw_body_invalid`, `plugin_raw_body_too_large` | 409 / 422 / 413 | `respond-bytes` problems. |
| `plugin_directory_unavailable` | 403 | `list-users` without a usable identity. |
| `plugin_execution_cycle` | 409 | A plugin would enter its own call chain, or the chain exceeds 16. |
| `plugin_failed` | 503 | The plugin trapped, timed out, ran out of memory or its backend does not load. |
| `plugin_owner_inactive` | 503 | The installer is no longer an active member. |

Errors from the record API (`permission_denied`, `validation_error`, `field_not_accessible`, ...) are described in [Errors](/docs/reference/errors/).


### Durable HTTP results and continuations

Set `on_response` to an event listed in the plugin manifest's `events`, and optionally supply a JSON `context`, on a deferred HTTP command. Every HTTP response, including non-2xx, is persisted and delivered to the same plugin through `onEvent` / `on_event`. The payload is `{ effect_id, context, response: { outcome, status, headers, body } }`; `body` is a byte array. Interpret the response body to decide business success. Use `ctx.effectResult(id)` / `ctx.effect_result(id)` to read the saved result; another plugin's result is never exposed.

Transport failure, timeout or a worker restart after sending produces `outcome: "unknown"`, a null status and `error: "plugin_http_outcome_unknown"`. This does not repeat the HTTP request automatically: reconcile the remote state from the continuation. A crash before storing a received response is also unknown. Saving the result and enqueuing the continuation share a transaction. Continuations retry independently, use a stable execution namespace and retain the originating authority; use deferred effects inside them. Sequential continuations are capped at 16 execution-chain entries. Without `on_response`, the result is still saved for polling, but no continuation is scheduled. HTTP responses are terminal transport results, including non-2xx; retry or reconcile explicitly from plugin logic. Response data may contain sensitive business information and is stored in the database; do not use this channel to fetch secrets.


### Payment settlement helpers

The Rust SDK's `payments::PaymentLedger` and JavaScript SDK's `PaymentLedger` provide pure settlement rules for integer minor-unit amounts. Initialize with sale id, currency and total, apply authoritative charge snapshots, then read `summary()`. The identity is `(provider, account, id)` and each snapshot has `sale_id`, `revision`, `currency`, `amount`, `refunded`, and normalized `status` (`pending`, `captured`, `failed`, `canceled`). Refunded captures retain `captured` status and increase `refunded`. Duplicate revisions are no-ops; conflicting equal revisions fail; older revisions are ignored; captured amounts and confirmed refunds cannot be erased. Rebuild from `snapshots()` when reloading persisted state.

`can_complete` requires exact net coverage with no pending charges. Overpayments require review, not automatic change. The helper does not authenticate webhooks, query a provider, assign authoritative revisions, persist state, handle disputes or guarantee transactionality. Use compare-and-swap and atomic transition/outbox persistence in the payment integration; never derive revisions from webhook arrival order. Reconcile late captures and refunds even after a sale has completed.
