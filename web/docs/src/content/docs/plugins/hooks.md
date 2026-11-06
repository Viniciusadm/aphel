---
title: Hooks
description: Run plugin code inside record writes and domain operations to validate, veto or change data, and learn the rules for side effects.
sidebar:
  order: 6
---

A hook is a point in Aphel's pipeline where your backend is called synchronously. Hooks let a plugin reject a write, change values before they are saved, or react inside the same transaction. They are the only place where you can stop an operation.

You subscribe in the manifest and answer in the backend's `on-hook` handler.

```toml
[[hooks.subscribe]]
hook = "record.validating"
record_types = ["library.book", "people.person"]
priority = 40
```

| Key | Meaning |
| --- | --- |
| `hook` | One of the record hooks below, or a domain hook from the host catalog. |
| `record_types` | Record types the subscription applies to. `"*"` matches all. At least one is required. |
| `priority` | Integer, default `50`. Lower runs first. |

The list of hooks this Aphel offers is available at `GET /api/v1/wasm-runtime/host-catalog` (permission `system.manage`). Each entry has the key, the owning module, the record type, the kind and whether the module is enabled. A hook of a disabled module never fires.

## Record hooks

The nine record hooks apply to every record type, built-in or from plugins.

| Hook | Kind | When | What you can do |
| --- | --- | --- | --- |
| `record.saving` | filter | Before values are validated and written, on create and update. | Return a patch of `values` and `custom_fields`. Throw to cancel. |
| `record.validating` | veto | After the write, before the commit. | Throw to roll the write back. |
| `record.saved` | action | After the write, inside the transaction. | Defer effects. Throw to roll back. |
| `record.deleting` | veto | Before a record is deleted. | Throw to block the deletion. |
| `record.archiving` | veto | Before a record is archived. | Throw to block it. |
| `record.deleted` | action | After a record was deleted, inside the transaction. | Defer effects. |
| `record.restored` | action | After a deleted record was restored from the trash, inside the transaction. | Defer effects. |
| `record.purged` | action | After a record was permanently deleted, inside the transaction. | Defer effects. |
| `record.committed` | action | After the transaction that wrote the record committed. | Defer effects. Runs later, as a durable job; an error cannot undo the write and makes the job retry. |

Kinds mean the following. A **veto** or **action** that returns an error stops the operation and the transaction is rolled back; the error is returned to whoever made the request. A **filter** may return a patch and may also fail. There is no way to ignore an error from the plugin: if the handler fails, the write fails.

### Order

Handlers run in this order for a given hook.

1. Handlers of other modules run by their priority. For `record.validating`, for example: line items (4), workflow (5), approvals (10), workflow (20), validation rules (30), leads (40) and then plugins (50).
2. All plugins run together at priority 50. Among plugins, the subscription `priority` decides (lower first), then the plugin key in alphabetical order.
3. For `record.saving`, each plugin receives the values as changed by the plugins before it.

A plugin that is enabled but whose backend does not load still takes part in the order. Its subscribed operations fail with `503 plugin_failed`, which blocks writes to the record types it hooks until the plugin is reinstalled. This is deliberate: a broken validator must not let invalid data through.

### Payload

The backend receives one call object.

| Field | Type | Description |
| --- | --- | --- |
| `hook` | string | The hook key, for example `record.validating`. |
| `record_type` | string | For example `library.book`. |
| `record_id` | string | The record id. For a create, the id the record will have. |
| `mode` | `"create"`, `"update"` or none | None for delete, archive, restore and purge hooks. |
| `actor` | object | `{ kind, id, user_id }`. `kind` is `user`, `api_token`, `automation`, `plugin`, `system` or `anonymous`. |
| `changes` | list | `{ field, old, new }` for each changed field. Custom fields use `custom_fields.<key>`. Empty in `record.saving`. |
| `values` | object | In `record.saving`: the draft values. In validating, saved and committed: the values of the write. `{}` for delete-type hooks. |
| `custom_fields` | object | In `record.saving`: the draft custom field values. `{}` in other hooks. |

Values and changes of fields the actor may not see are removed before the call (field permissions apply to plugins).

The backend can also call `hook-state-json` (JavaScript `ctx.hookState()`) to read a snapshot of the record.

```json
{
  "operation_id": "0192f0c4-0000-7000-8000-00000000000a",
  "record_type": "library.book",
  "record_id": "0192f0c4-7d0e-7a8a-9f3e-3c2b1a0d9e8f",
  "phase": "record.validating",
  "before": { "values": {}, "custom_fields": {} },
  "before_available": true,
  "proposed": { "values": {}, "custom_fields": {} },
  "persisted": null,
  "effects_allowed": true
}
```

`before` is the record before the write (null for a create), `proposed` the record as it will be written and `persisted` the committed record in `record.committed`. `effects_allowed` is false when the hook runs read-only (see below).

### Changing values with `record.saving`

Return objects with the fields to change. They are merged into the draft, key by key.

```js
onHook(ctx, call) {
  if (call.is('record.saving', 'library.book') && typeof call.values.title === 'string') {
    return { values: { title: call.values.title.trim() } }
  }
  return {}
}
```

Rules:

- The outcome is two JSON objects, `values` and `custom_fields`. Anything else fails with `422 plugin_hook_patch_invalid`.
- Every key must be an active, editable field of the record type. Otherwise the write fails with `403 plugin_field_not_writable`: `the hook cannot change field <key>`.
- Patches are checked only in `record.saving`. In other hooks the returned values are ignored.
- The merged values go through the normal field validation afterwards, so a patch cannot bypass required fields, types or limits.

### Rejecting with errors

Throw (JavaScript) or return `Err` (Rust) a plugin error to cancel the operation.

```js
throw PluginError.field('invalid_isbn', 'isbn', 'An ISBN has 10 or 13 digits.')
```

A plugin error has a `code`, a `message`, an optional HTTP `status` and a list of `fields`.

| You set | The caller receives |
| --- | --- |
| `fields` not empty | `422` with your `code`, your `message` and the field list. |
| `status` 401, 403, 404, 409, 413, 429 or 503 | That status with your `code` and `message`. |
| Any other or no `status` | `422` with your `code` and `message`. |

A code must be 1 to 64 characters of lowercase letters, digits and `_`. Any other code is replaced by `plugin_error`. Plugin codes can be translated in the plugin's own web namespace as `errors.<code>`.

An uncaught exception in JavaScript is converted by the SDK into a plugin error with the code `plugin_error`, status `422` and the exception message. A panic or trap in Rust, a timeout (2 seconds for hooks) and an out-of-memory trap are not plugin errors: the call fails with `503 plugin_failed`. Use explicit plugin errors for expected rejections.

## Domain hooks

Built-in modules also publish hooks for their own operations. They are listed in the host catalog with the record type they apply to. Domain hooks use the same handler, the same subscription syntax and the same errors.

| Hook | Record type | Kind | Values |
| --- | --- | --- | --- |
| `activity.completing` | `tasks.task` | veto | `{ activity, previous, type_key }` |
| `automation.action_running` | any | veto | `{ automation_id, run_id, step_id, action, record: { record_type, record_id }, config, dry_run }` |
| `deal.losing` | `deals.deal` | veto | `{ deal, previous, stage, lost_reason }` |
| `deal.lost` | `deals.deal` | action | `{ deal, previous, stage, lost_reason }` |
| `deal.stage_changing` | `deals.deal` | veto | `{ deal, previous, from, to }` |
| `deal.winning` | `deals.deal` | veto | `{ deal, previous, stage, lost_reason }` |
| `deal.won` | `deals.deal` | action | `{ deal, previous, stage }` |
| `email.outgoing` | `email` or the linked record's type | filter | `{ mailbox_id, sender_id, from, to, cc, bcc, subject, body: { text, html }, record, thread_id, scheduled, tracking }`. Return `{ subject?, body?: { text?, html? } }` to change them; HTML is sanitized. |
| `lead.converted` | `people.person` | action | `{ lead, deal_id, company_id }` |
| `lead.converting` | `people.person` | filter | `{ contact_id, lead, deal, company }`. Return `{ deal?, company? }` to merge field values into the deal or company; returning a company object makes the conversion create a company when the contact has none. |
| `lead.disqualifying` | `people.person` | veto | `{ lead, reason: { id, key, name }, note }` |
| `lead.qualifying` | `people.person` | veto | `{ lead }` |
| `quote.accepting` | `quotes.quote` | veto | `{ quote, deal_id, acceptor: { name, email }, manual }` |
| `sequence.step_running` | `people.person` | veto | `{ enrollment_id, sequence_id, contact_id, enrolled_by, position, kind, template_id }` |

In the call object of a domain hook, `values` carries the payload above, `custom_fields` is `{}`, `mode` is empty and `changes` is empty. Filter hooks merge the `values` you return into the payload; plugins that run later see the merged payload. For veto hooks, throw to block. For `sequence.step_running`, the sequences module treats a definitive error as a reason to remove the contact from the sequence and a transient error as a reason to retry the step.

The generated catalog lists these hooks with the exact record types. The reference [Hooks catalog](/docs/reference/hooks-catalog/) lists the hooks of native modules.

## Side effects inside hooks

Hooks run inside the database transaction of the write, with a two second limit. Two rules follow from that.

1. **Record writes are refused.** Calling `create-record`, `update-record`, `delete-record`, `update-record-if` or `apply-records` from an in-transaction hook fails with `409 plugin_write_in_hook`: `plugins cannot execute side effects inside a record hook; use a durable event handler`. The same code is returned for key-value writes, uploads, `notify`, `emit`, `http-fetch` and for writing operations called with `invoke`. Reads are allowed: `get-record`, `list-records`, `aggregate-records`, `kv-get`, `kv-list`, `read-file`, `secret`, `invoke` of read-only operations, and so on.
2. **Defer effects instead.** `defer-effect` stores a command with the hook's transaction. If the write commits, a durable worker job runs the command later with the authority captured when you deferred it; if the write is rolled back (including by your own veto), the effect disappears. Effect keys make the command idempotent. See [Host API](/docs/plugins/host-api/#defer-effect).

```js
onHook(ctx, call) {
  if (call.is('record.saved', 'library.book') && call.isCreate && call.actor.userId) {
    ctx.deferEffect(`welcome:${call.recordId}`, {
      kind: 'notify',
      name: 'new_book',
      recipient_user_id: call.actor.userId,
      title: 'Book added',
      record_type: 'library.book',
      record_id: call.recordId,
      data: {},
    })
  }
  return {}
}
```

The example assumes the manifest declares `[[notifications]]` with `name = "new_book"`. A deferred `notify` of an undeclared notification is refused with `plugin_notification_not_declared`.

`record.committed` runs after the commit as a job, but it is still called through the hook interface with the same two second limit and the same restrictions: writes go through `defer-effect`.

### Read-only re-entry

When a hook is triggered by a write that the same plugin made earlier in the same execution chain (for example, a deferred effect of the plugin creates a record the plugin also hooks), the hook still runs, but read-only: `effects_allowed` is false, and any attempt to defer or perform an effect fails with `403 plugin_read_only`. The exception is `record.committed`. This prevents a plugin from reacting to its own writes in a loop. An execution chain also stops with `409 plugin_execution_cycle` when it would enter the same plugin twice outside a hook or exceed 16 plugins.

## Writing hook handlers well

- Keep them fast. A hook has two seconds, including the time to read records.
- Prefer `record.validating` for checks that need the final values and `record.saving` for normalization.
- Do not depend on the order of other plugins. If two plugins change the same field in `record.saving`, the later one wins. Installing a plugin that subscribes to `record.saving`, `record.validating`, `record.deleting` or `record.archiving` for a record type that another enabled plugin also hooks requires an explicit acknowledgement; see [Lifecycle](/docs/plugins/lifecycle/).
- Make the handler tolerate missing keys: `values` may not contain every field, and fields hidden from the actor are always absent.
- Return an empty outcome when you have nothing to say.
