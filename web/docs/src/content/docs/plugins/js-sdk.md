---
title: JavaScript SDK
description: Write a plugin backend in JavaScript or TypeScript with @aphel/plugin-sdk, from scaffold and build to the handler and context API.
sidebar:
  order: 13
---

`@aphel/plugin-sdk` lets you write the backend of a plugin in JavaScript or TypeScript. You export handlers with `definePlugin`, call the [host API](/docs/plugins/host-api/) through a `ctx` object, and build the result into `backend.wasm` with `aphel-plugin-js`.

The package lives in the Aphel repository at `sdk/js/aphel-plugin-sdk` and is used through a `file:` dependency. It is not published to npm.

## Setup

1. Build the command line tool once (see [Your first plugin](/docs/plugins/first-plugin/)).
2. Scaffold a project:

```bash
aphel-plugin new library --js --sdk /path/to/aphel/sdk/js/aphel-plugin-sdk
cd aphel-library
```

The command creates `aphel-<key with dashes>/` with:

| File | Content |
| --- | --- |
| `plugin.toml` | A minimal manifest: key, name, version, `api = 1`, `requires_aphel`, `[menu] flat = true` and a `GET /ping` route. |
| `package.json` | `private`, `type: module`, the script `build:backend` and the `file:` dependency on the SDK. |
| `.gitignore` | `node_modules/`, `backend.wasm`, `*.aphelplugin`. |
| `src/plugin.js` | A plugin with one route that answers `{ "pong": "<path>" }`. |

`package.json` defines the build script the tool looks for:

```json
{
  "scripts": {
    "build:backend": "aphel-plugin-js src/plugin.js -o backend.wasm"
  },
  "devDependencies": {
    "@aphel/plugin-sdk": "file:/path/to/aphel/sdk/js/aphel-plugin-sdk"
  }
}
```

## Build

| Command | What it does |
| --- | --- |
| `aphel-plugin build` | Runs `npm install` if `node_modules` is missing, then `npm run build:backend`. If `web/package.json` has a `build` script it builds the web bundle too. |
| `aphel-plugin pack` | Does what `build` does, then writes `<key>-<version>.aphelplugin`. Use `-o file.aphelplugin` for another name. |
| `npx aphel-plugin-js <entry.js\|entry.ts> -o backend.wasm` | Builds only the backend. |

`aphel-plugin-js` bundles the entry file with esbuild (ES2022, no Node built-ins, the `aphel:*` and `wasi:*` imports left external) and turns the bundle into a WebAssembly component with `componentize-js`, using the contract `aphel:plugin`. It prints the output size. TypeScript entry files are compiled by the bundler without type checking; run `tsc --noEmit` yourself.

The entry module must export the eight handlers by name, or default-export an object with them:

```js
import { definePlugin } from '@aphel/plugin-sdk'

export const { onHook, onEvent, onRoute, onSchedule, onAction, onWidget, onOperation, onContribution } =
  definePlugin({})
```

`definePlugin` always returns all eight, so the line above works whether you implement some handlers or all.

## Runtime model

- Each call creates a fresh instance and runs your module from the top, so the cost of imports and top-level code is paid on every call. Keep dependencies small. State does not survive between calls: use records or key-value storage.
- All host calls are synchronous. Do not use `async`/`await`, timers or `fetch`: they are not available. Use `ctx.http` for requests.
- There is no file system, environment, network or Node.js API.
- A component embeds a JavaScript engine, so a JavaScript `backend.wasm` is much larger than a Rust one. The build prints its size. Packages are limited to 50 MB in total.

## Handlers

Every handler receives `ctx` first. Handlers are optional; a missing one does nothing, except `onRoute`, which answers `404`.

```ts
definePlugin({
  onHook(ctx, call) {},
  onEvent(ctx, event) {},
  onRoute(ctx, request) {},
  onSchedule(ctx, name, slot) {},
  onAction(ctx, call) {},
  onWidget(ctx, request) {},
  onOperation(ctx, key, input) {},
  onContribution(ctx, point, name, input) {},
})
```

| Handler | Argument | Return |
| --- | --- | --- |
| `onHook` | `HookCall`: `hook`, `recordType`, `recordId`, `mode`, `actor`, `changes`, `values`, `customFields`, and helpers `is(hook, recordType)`, `changed(field)`, `customChanged(field)`, `isCreate` | `{ values?, customFields? }` or nothing |
| `onEvent` | `PluginEvent`: `id`, `type`, `recordType`, `recordId`, `actor`, `occurredAt`, `payload` | nothing |
| `onRoute` | `Request`: `method`, `path`, `route`, `params`, `query`, `headers`, `body`, `actor`, and helpers `is(method, route)`, `header(name)` | `{ status, body }`; nothing gives `404 route_not_found` |
| `onSchedule` | `name`, `slot` | nothing |
| `onAction` | `ActionCall`: `name`, `input`, `recordType`, `recordId`, `record`, `automationId`, `runId`, `actor` | JSON value |
| `onWidget` | `WidgetRequest`: `name`, `settings`, `periodFrom`, `periodTo`, `timezone`, `actor` | JSON value |
| `onOperation` | `key`, `input` (parsed JSON) | JSON value |
| `onContribution` | `point`, `name`, `input` | JSON value |

Notes:

- `actor` is `{ kind, id, userId }` with `kind` one of `user`, `api_token`, `automation`, `plugin`, `system`, `anonymous`.
- `changes` is a list of `{ field, old, new }` with parsed JSON values.
- `request.headers` is a list of `[name, value]`; `request.header(name)` is case-insensitive in effect because names arrive in lowercase.
- Returning `undefined` from `onAction`, `onWidget`, `onOperation` or `onContribution` produces `null`.

Response helpers: `ok(body)` (200), `created(body)` (201) and `respond(status, body)`. Constants: `hooks` (`SAVING`, `VALIDATING`, `SAVED`, `DELETING`, `ARCHIVING`, `COMMITTED`, `DELETED`, `RESTORED`, `PURGED`), `domainHooks` (for example `DEAL_WON`) and `operations` (for example `TASKS_CREATE`).

## The context

`ctx` is the same object in every handler. Its methods map one to one onto the host functions; arguments and results are plain JavaScript values.

| Member | Host function |
| --- | --- |
| `ctx.records.get(type, id)` | `get-record` (returns `null` when missing) |
| `ctx.records.list(type, { filter, sort, search, page, perPage })` | `list-records`, returns `{ data, total }` |
| `ctx.records.create(type, values)`, `update(type, id, values)`, `delete(type, id)` | `create-record`, `update-record`, `delete-record` |
| `ctx.records.updateIf(type, id, expectedUpdatedAt, values)` | `update-record-if` |
| `ctx.records.apply([{ recordType, id?, expectedUpdatedAt?, values }])`, `ctx.records.ref(n)` | `apply-records` and `{ $ref: n }` |
| `ctx.records.aggregate(query)` | `aggregate-records` |
| `ctx.records.iterate(type, options, cursor)` | `iterate-records`, returns `{ data, cursor }` |
| `ctx.kv(collection)` | `get`, `set`, `delete`, `list` (returns `[{ id, value }]`), `increment(id, delta = 1)`, `setIfAbsent` |
| `ctx.nextNumber(sequence)` | `next-number` |
| `ctx.http.send({ method, url, headers, body, json })`, `ctx.http.get(url)` | `http-fetch` |
| `ctx.notify({ kind, recipientUserId, title, body, record, link, data, dedupeKey })` | `notify` (`record` is `{ type, id }`) |
| `ctx.emit(name, record, payload)` | `emit` (`record` is `{ type, id }` or null) |
| `ctx.invoke(operation, input)` | `invoke` |
| `ctx.deferEffect(key, command)` | `defer-effect` |
| `ctx.settings()`, `ctx.setting(key)` | `settings-json` |
| `ctx.secret(key)` | `secret`, returns `null` when unset |
| `ctx.can(permission)`, `ctx.permissions()` | `can`, `permissions` |
| `ctx.uploadFile(type, id, filename, body)`, `ctx.readFile(id)`, `ctx.respondBytes(contentType, filename, body)` | `upload-file`, `read-file` (returns `{ file, body }`), `respond-bytes` |
| `ctx.setAuditReason(reason)` | `set-audit-reason` |
| `ctx.organization()`, `ctx.listUsers(query)`, `ctx.listRoles()`, `ctx.listTeams()` | the directory functions |
| `ctx.hookState()`, `ctx.executionContext()` | `hook-state-json`, `execution-context-json` |
| `ctx.now()`, `ctx.log(level, message)`, `ctx.info(message)` | `now`, `log` |

Bodies for `ctx.http` accept strings and `Uint8Array`; `json` serializes a value and sets `content-type: application/json`. The response has `status`, `ok`, `headers`, `header(name)`, `body` (bytes), `text()` and `json()`.

Which functions are allowed in which handler is explained in [Host API](/docs/plugins/host-api/#where-side-effects-are-allowed). The types of every shape are in `src/index.d.ts` of the SDK: `Ctx`, `HookCall`, `PluginEvent`, `Request`, `Response`, `ActionCall`, `WidgetRequest`, `EffectCommand` and the rest.

## Errors

`PluginError` carries a code, a message, an optional HTTP status and a list of field errors.

```js
import { PluginError } from '@aphel/plugin-sdk'

throw new PluginError('stock_empty', 'There are no copies left.', { status: 409 })
throw PluginError.field('invalid_isbn', 'isbn', 'An ISBN has 10 or 13 digits.')
throw PluginError.notFound('book_not_found', 'No such book.')
throw PluginError.forbidden('not_allowed', 'You cannot do that.')
```

- Errors raised by the host (a denied permission, a failed validation, `plugin_write_in_hook`) are thrown as `PluginError` with the host's code, status and fields. You can catch and inspect them: `error.code`, `error.status`, `error.fields`.
- An error that you let escape is returned to the caller as it is. See the mapping in [Hooks](/docs/plugins/hooks/#rejecting-with-errors).
- Any other exception, for example a `TypeError` from a bug, is converted into `PluginError('plugin_error', message)` with status `422`. Fix the bug instead of relying on it.
- Codes must be 1 to 64 characters of lowercase letters, digits and `_`, otherwise the caller sees `plugin_error`.

## TypeScript

Types ship with the package (`src/index.d.ts`). A typed handler:

```ts
import { definePlugin, hooks, type HookCall, type Ctx } from '@aphel/plugin-sdk'

export const { onHook, onEvent, onRoute, onSchedule, onAction, onWidget, onOperation, onContribution } =
  definePlugin({
    onHook(ctx: Ctx, call: HookCall) {
      return call.is(hooks.SAVING, 'library.book') ? { values: { title: String(call.values.title).trim() } } : {}
    },
  })
```

Point the build script at the `.ts` entry: `aphel-plugin-js src/plugin.ts -o backend.wasm`.

## Size and performance

- The host compiles the component when the package is installed and caches the compiled code by the hash of `backend.wasm`, in memory and in the database, so restarts and other processes reuse it.
- Every call instantiates a new engine. Hooks have two seconds in total, so keep their work small and avoid large dependencies.
- Avoid loading the whole table: use `perPage` and filters, or `iterate`.
- Memory is limited to 64 MB per call. Large JSON values and big `Uint8Array` bodies count against it.
