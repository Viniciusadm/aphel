---
title: Events
description: Subscribe to domain events, understand delivery guarantees and retries, and use deferred effects and idempotency keys safely.
sidebar:
  order: 7
---

Every business mutation in Aphel publishes a domain event in the same transaction as the change. A plugin can subscribe to events in the manifest and handle them in `on-event`. Events are the right place for work that happens after a change: notifying someone, calling an external service, creating follow-up records.

Use [hooks](/docs/plugins/hooks/) instead when you must decide whether a change is allowed.

## Subscribing

List the event types in the top-level `events` key.

```toml
events = ["library.book.created", "library.book.updated", "plugin.library.*"]
```

| Pattern | Matches |
| --- | --- |
| `library.book.created` | Exactly that event type. |
| `library.book.*` | Every event type that starts with `library.book.`, including deeper levels. |

The installer does not check the names, so a typo silently subscribes to nothing. A plugin with `events` needs a backend.

Event types you can receive:

| Source | Event types |
| --- | --- |
| Records of your entities | `<plugin key>.<entity>.created`, `.updated`, `.deleted`, `.archived`, `.unarchived` |
| Records of other types and domain events | The types published by Aphel's modules, for example `<record type>.created` and `.updated`. See [Webhooks and events](/docs/reference/webhooks-and-events/) for the catalog. |
| Events emitted by plugins | `plugin.<plugin key>.<name>`, declared with `[[emits]]` |
| Plugin lifecycle | `plugin.installed`, `plugin.uninstalled`, `plugin.enabled`, `plugin.disabled`, `plugin.settings_updated`, `plugin.grants_reapplied`, with the payload `{ "plugin": "<key>" }`. A subscription to `plugin.*` also matches these. |

Update events carry the changed fields in `payload.changes`, a list of `{ field, old, new }`.

## The event envelope

`on-event` receives one event.

| Field | JavaScript | Rust | Description |
| --- | --- | --- | --- |
| `id` | `event.id` | `event.id` | Unique, stable id of the event. The same event always has the same id, including retries. |
| `event-type` | `event.type` | `event.event_type` | The event type. |
| `record-type`, `record-id` | `event.recordType`, `event.recordId` | `event.record_type`, `event.record_id` | The record the event is about, or empty. |
| `actor` | `event.actor` | `event.actor` | Who caused it: `{ kind, id, user_id }`. |
| `occurred-at` | `event.occurredAt` | `event.occurred_at` | RFC 3339 timestamp. |
| `payload` | `event.payload` | `event.payload` | JSON payload of the event. |

```js
onEvent(ctx, event) {
  if (event.type !== 'library.book.created') return
  ctx.deferEffect(`created:${event.id}`, {
    kind: 'kv_set',
    collection: 'created_books',
    id: event.recordId,
    value: { at: event.occurredAt },
  })
}
```

## Who the plugin acts as

An event has no signed-in user. The handler runs as the plugin, with the permissions of the user who installed it, limited to the plugin's capabilities. If that user is no longer an active member, every event fails with `503 plugin_owner_inactive` until an administrator reinstalls the plugin. See [Security model](/docs/plugins/security-model/).

## Delivery

Events go through the outbox: the event is stored with the change, then a worker delivers it. The plugin runtime creates one job per plugin and event, deduplicated by the event id, and runs `on-event` with a 30 second limit.

- **At least once.** If the handler fails, throws, times out or the worker stops before the result is recorded, the job is retried (five attempts by default, with a growing delay). A handler can therefore see the same event more than once. Make it idempotent.
- **Completed means done.** When a handler returns successfully, the execution is marked completed and the event is never delivered to that plugin again.
- **Disabled plugins.** If the plugin is disabled or uninstalled before the job runs, the job fails with `503 plugin_disabled` and is retried until its attempts run out. Events that happen while a plugin is disabled are not queued for it.
- **No ordering guarantee** between events, even for the same record.
- The last error of a failed execution is stored with it. See [Testing and debugging](/docs/plugins/testing-and-debugging/) for how to investigate failures.

### A plugin never receives events it caused

An event is not delivered to a plugin when the actor of the event is that plugin, or when the plugin is already part of the execution chain that produced the event. A plugin that creates a record in an event handler does not receive the `created` event of that record. Other plugins and automations do.

## Side effects in event handlers

Because an event handler is retried, direct side effects are refused. In `on-event` (and in schedules, automation actions and privacy requests):

- `create-record`, `update-record`, `update-record-if`, `delete-record`, `apply-records`, key-value writes, `next-number`, `upload-file`, `notify`, `emit` and `http-fetch` fail with `409 plugin_effect_required`: `retryable plugin handlers must use defer-effect for side effects`.
- Writing operations called with `invoke` fail with the same code. Read-only operations and external queries are allowed.
- Reads are allowed.

Use `defer-effect` instead. A deferred effect is stored durably and run by a worker with the authority captured at the time. Effects are deduplicated by key, and a command that already committed is not repeated when the worker retries it.

### Idempotency keys

Each deferred effect has a key, 1 to 128 bytes, chosen by you. Within one execution of a handler:

- Deferring the same key with the **same command** returns the id of the first effect and creates nothing new.
- Deferring the same key with a **different command** fails with `409 plugin_effect_key_conflict`.

An execution keeps its identity across retries of the same event, so keys derived from the event id (`"welcome:" + event.id`) make a retried handler defer nothing twice. Use the id of the thing you act on when one event may lead to several effects of the same kind.

A single execution can defer at most 100 distinct effects (`plugin_effect_limit` beyond that). The commands you can defer are listed in [Host API](/docs/plugins/host-api/#defer-effect).

HTTP effects send an `Idempotency-Key` header equal to the effect id, so a retried request can be recognized by the remote service. See [Host API](/docs/plugins/host-api/).

## Emitting events

A plugin publishes its own events with the host function `emit`, after declaring them with `[[emits]]`. They are named `plugin.<plugin key>.<name>`, appear in the catalog of events available to webhooks with the payload you send, and can be used as automation triggers. See [Automation and widgets](/docs/plugins/automation-and-widgets/).

`emit` is refused in hooks and in retryable handlers; defer an `emit` effect there, or call it from a route.

## The change feed

For web bundles that want live updates, Aphel keeps a change feed per plugin: every create, update, delete, archive and restore of the plugin's entities and every `plugin.<key>.*` event, in commit order.

| Endpoint | Description |
| --- | --- |
| `GET /api/v1/wasm-runtime/{key}/changes?since=<seq>&limit=<n>` | Entries after `since` (default 0). `limit` up to 500. Response: `{ data: [{ seq, event_type, record_type, record_id, fields, occurred_at }], next, reset }`. |
| `GET /api/v1/wasm-runtime/{key}/changes/stream` | The same entries as Server-Sent Events: event `change` with `id` equal to `seq`, event `reset` and event `error`. Resumes from `since` or the `Last-Event-ID` header. The server closes the stream after five minutes. |

Entries of record types the caller cannot read are skipped, `fields` lists names of changed fields but never values, and the feed keeps 24 hours. `reset: true` means entries after `since` were pruned and the client must reload its state. Both endpoints answer `404 plugin_not_found` for plugins that are not installed or are disabled. See [Web bundle](/docs/plugins/web-bundle/) for the client helper.
