---
title: Automation, notifications and widgets
description: Add automation actions and triggers, notification kinds, dashboard widgets, provided operations, contributions and privacy handlers.
sidebar:
  order: 11
---

Besides data and hooks, a plugin can plug into several built-in features: automations, notifications, the dashboard, the operation catalog and privacy requests. Each section below shows the manifest declaration and the backend handler.

## Automation actions

An action appears in the automation builder next to Aphel's own actions, and runs your backend when an automation reaches it.

```toml
[[actions]]
key = "send_to_catalog"
label = "Send to catalog"
description = "Publishes the record to the external catalog."
record_types = ["library.book"]
retry_safe = true

[[actions.inputs]]
key = "channel"
label = "Channel"
type = "select"
options = ["web", "print"]
required = true

[[actions.inputs]]
key = "note"
label = "Note"
type = "text"
```

The action is registered as `plugin.<plugin key>.<action key>`. See the [manifest](/docs/plugins/manifest/#actions-emits-notifications-widgets) for limits: up to 20 actions with up to 20 inputs each.

| Property | Behavior |
| --- | --- |
| `record_types` | Record types the action can run on. Empty means every record type that automations can use. |
| `retry_safe` | Tells the automation engine the action can be retried after a failure without repeating its effect. Default `false`. |
| `inputs[].type` | `text` (string), `number`, `boolean` or `select` (string from `options`). Default `text`. |

The configuration the builder produces is validated before it is saved: an unknown input is rejected (`is not an input of this action`), a missing required input gives `is required`, and a value of the wrong type gives `must be a valid <type>`.

`on-action` receives:

| Field | JavaScript | Description |
| --- | --- | --- |
| `name` | `call.name` | The action key as declared (without the `plugin.<key>.` prefix). |
| `input` | `call.input` | The validated configuration object. |
| `record-type`, `record-id` | `call.recordType`, `call.recordId` | The record the automation runs on. |
| `record-json` | `call.record` | The record as JSON. |
| `automation-id`, `run-id` | `call.automationId`, `call.runId` | The automation and the run. |
| `actor` | `call.actor` | The automation as actor. |

Return any JSON value. It is recorded as the result of the step; a result that is not valid JSON is recorded as `null`.

```js
onAction(ctx, call) {
  if (call.name === 'send_to_catalog') {
    ctx.deferEffect(`catalog:${call.runId}:${call.recordId}`, {
      kind: 'http',
      method: 'POST',
      url: 'https://catalog.example.com/books',
      body: JSON.stringify({ channel: call.input.channel, book: call.record.title }),
    })
    return { queued: true }
  }
}
```

The action runs with the permissions of the user who published the automation version, limited to the plugin's capabilities, as the actor `automation`. It has 30 seconds and runs in a retryable context, so side effects go through `defer-effect` (see [Events](/docs/plugins/events/#side-effects-in-event-handlers)). Use the run id in effect keys so a retried step does not repeat the effect. The execution is also recorded per automation, run and step.

If the plugin is disabled, uninstalled or its backend cannot load, the step fails with `503 plugin_unavailable` or `503 plugin_failed`.

## Automation triggers

Every `[[emits]]` entry becomes a trigger in the automation builder.

```toml
[[emits]]
name = "book_borrowed"
label = "Book borrowed"
description = "A member borrowed a book."
record_types = ["library.book"]
```

The backend fires it with `emit` after declaring it.

```js
ctx.emit('book_borrowed', { type: 'library.book', id: book.id }, { member: memberId })
```

The event is `plugin.library.book_borrowed`. Rules:

- The name must be declared (`404 plugin_event_not_declared`; `403` when deferred).
- The payload must be a JSON object of at most 64 KiB.
- The record is optional but `record-type` and `record-id` go together, the record must be readable by the plugin, and its type must be in `record_types` when that list is not empty.
- A trigger only starts automations when the event carries a record of a type the trigger supports.
- The same event is available to webhooks and can be consumed by other plugins in `events`.

`emit` is refused in hooks and retryable handlers; defer an `emit` effect there.

## Notifications

Declare notification kinds so users can choose how they receive them.

```toml
[[notifications]]
name = "overdue"
label = "Book overdue"
description = "A borrowed book is past its return date."
channels = ["in_app", "email"]
email_default = true
```

The kind is registered as `<plugin key>_<name>`. `channels` can contain `in_app`, `email` and `push` (default: all three); `email_default` and `push_default` are the initial preferences.

Send one with `notify`:

| Field | Rules |
| --- | --- |
| `kind` | The declared `name`. Otherwise `404 plugin_notification_not_declared`. |
| `recipient-user-id` | An active user. Otherwise `422 plugin_notification_recipient_invalid`. |
| `title` | 1 to 200 characters. |
| `body` | Optional, up to 2000 characters. |
| `record-type`, `record-id` | Optional pair. The recipient must be able to read the record (`403 plugin_notification_record_not_readable`). |
| `link` | Optional application path starting with a single `/`, up to 500 characters, for example `/deals/<id>`. |
| `data` | Optional JSON attached to the notification. |
| `dedupe-key` | Optional. Identifies the notification for deduplication, scoped to the plugin. Without it every call gets a unique key. |

Other violations give `422 plugin_notification_invalid`. `notify` is refused in hooks and retryable handlers, where you defer a `notify` effect: `{ kind: "notify", name, recipient_user_id, title, ... }`.

## Dashboard widgets

```toml
[[widgets]]
name = "reading_now"
title = "Books being read"
category = "library"
permission = "library.book.read"
size = "small"
uses_period = false
default_settings = { limit = 5 }
```

The widget key is `<plugin key>_<name>`. `permission` must be one of the plugin's capabilities and decides who sees the widget. `size` is `small`, `medium` (default) or `large`. A user's settings for the widget are normalized to a JSON object of at most 16 KB.

`on-widget(name, request)` receives `{ settings, period-from, period-to, timezone, actor }` and returns JSON data. In JavaScript the request also has `name`.

```js
onWidget(ctx, request) {
  if (request.name === 'reading_now') {
    const { total } = ctx.records.list('library.book', { perPage: 1, filter: { field: 'status', operator: 'eq', value: 'reading' } })
    return { total }
  }
}
```

Widgets run read-only, as the viewer limited to the plugin's capabilities, with a five second limit. Writes, `notify`, `emit`, `http-fetch` and `defer-effect` fail with `403 plugin_read_only`. Widgets are registered only for plugins that have a backend.

The backend returns data; the dashboard draws it with a web renderer registered for the widget key by the plugin's web bundle (see [Web screens](/docs/plugins/web-screens/)). Without a renderer the dashboard shows an unsupported-widget state.

## Provided operations

Aphel has a catalog of operations that modules expose, and plugins can both use them and add to them.

**Using operations.** List the operations you call in the top-level `operations` array and call them with `invoke` ([Host API](/docs/plugins/host-api/#module-operations)). The plugin acts with the intersection rules, so list the permissions each operation needs in `requires`.

**Providing operations.** A plugin can offer an operation to other plugins and modules:

```toml
[provides]
contracts = []

[[provides.operations]]
key = "library.lookup"
description = "Finds a book by ISBN."
effect = "read"
permission = "library.book.read"
idempotent = true
```

| Key | Description |
| --- | --- |
| `key` | Dotted lowercase key such as `library.lookup`, unique across Aphel. A key that a built-in module owns, or that another installed plugin provides, is refused. |
| `effect` | `read` (default), `write` or `external`. A `read` operation runs read-only. |
| `permission` | Permission the caller must hold. Checked against the caller's own permissions. |
| `idempotent` | Whether repeated calls with the same input are safe. |

The backend answers in `on-operation(key, input-json)` and returns JSON. The operation runs with the caller's permissions limited to the plugin's capabilities, with 30 seconds. Callers can use it with `invoke` once they declare it in their `operations`, and it shows up in the host catalog. Another plugin that lists your operation in its `operations` is protected: you cannot uninstall, disable or upgrade your plugin in a way that removes the operation while it is in use (`409 plugin_has_dependents`).

Errors callers may see: `404 plugin_operation_unknown` (the operation does not exist, or its module or plugin is disabled), `403 plugin_operation_not_declared` (the caller did not declare it), `503 plugin_operation_timeout`.

## Contributions

A contribution adds an item to an extension point published by a module.

```toml
[[contribute]]
point = "dashboard.widget"
name = "overdue"
title = "Overdue books"
data = { size = "small" }
```

`point` is the dotted name of an extension point, `name` is unique within that point, `title` is optional and `data` is static data given to the point. The module that owns the point calls your backend through `on-contribution(point, name, input-json)`, which returns JSON. A call for a contribution that the manifest does not declare answers `404 plugin_contribution_unknown`. Which points exist and what they pass and expect is defined by the module that owns them.

## Privacy requests

Privacy requests (export, anonymize, delete a data subject's data) are handled by Aphel for its own records and for your entities. If you keep personal data elsewhere, for example in key-value storage or an external service, opt in to be asked.

```toml
[privacy]
subject_data = true
```

The backend then receives `on-action` calls named `privacy.export`, `privacy.anonymize` and `privacy.delete`, with the `record-type` `privacy_request` and the request id as `record-id`. The `input` is:

```json
{
  "request_id": "0192f0c4-7d0e-7a8a-9f3e-3c2b1a0d9e8f",
  "action": "export",
  "records": [{ "record_type": "people.person", "id": "0192f0c4-0000-7000-8000-000000000002" }]
}
```

Return a JSON object. Put the number of affected items in `affected`; any other keys are included in the report of the request under the plugin's key. An error is counted as a failure of that source and does not stop the other sources. The call runs as the plugin with a 60 second limit, in a retryable context.

Mark fields of your entities `personal` or `sensitive` so Aphel's own privacy tooling handles them. See [Entities and fields](/docs/plugins/entities-and-fields/).
