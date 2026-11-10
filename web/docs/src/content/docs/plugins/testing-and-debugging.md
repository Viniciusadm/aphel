---
title: Testing and debugging
description: How to iterate on a plugin locally, read the consent output, and diagnose a web bundle, a stylesheet or an installation that fails.
sidebar:
  order: 56
---

There is no hot reload for plugins. The loop is: build, pack, upload, reload. This page shows how to make that loop fast and what to check when something goes wrong.

## The local loop

1. Run Aphel locally and create an API token with the `system.manage` scope (Account, API tokens).
2. After each change, rebuild and install:

```bash
aphel-plugin pack . -o /tmp/deal_watch.aphelplugin
curl -s -X POST 'http://127.0.0.1:3001/api/v1/wasm-runtime?acknowledge_web_version=0.1.0' \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -F package=@/tmp/deal_watch.aphelplugin
```

3. Reload the browser page. Bundles are imported once, when the application starts, so a reload is always required. The Plugins page does it for you; the API does not.

Notes:

- `acknowledge_web_version` must equal the `version` in `plugin.toml`. You do not need to bump the version on every iteration, since reinstalling the same version is accepted.
- Install replaces the plugin in place and keeps its settings and enabled state. While you iterate, a disabled plugin stays disabled.
- Run the type checker before packing (`npm run typecheck` in `web/`). The bundler does not check types.
- To test a bundle in isolation from other plugins, disable the others.

## Reading the consent dry run

`POST /api/v1/wasm-runtime?dry_run=true` validates the package exactly like a real install and changes nothing. Use it as a linter for the manifest and the package. The fields most useful during development:

| Field | What to check |
| --- | --- |
| `has_web` | True when `[web]` is declared. The real install will then need `acknowledge_web_version`. |
| `ui` | The interface label, or null when the plugin has no `[ui]`. |
| `public_routes` | Includes `GET /p/<key>/` when there is a public app. |
| `permissions`, `requires` | What the plugin asks for. |
| `replaces` | Built-in modules that this plugin will hold disabled. |
| `conflicts`, `module_conflicts` | Overlaps that need `acknowledge_conflicts=true`, or that block the install. |
| `installed_version` | The version being replaced, null for a first install. |

## Where to look when something does not work

### The browser console

A bundle that fails to import prints its error with `console.error`, and users see a notice titled "Part of the system did not load" with the plugin name. Open the developer tools, reload and read the first error. Typical causes:

| Message or symptom | Cause and fix |
| --- | --- |
| `plugin <key> does not export its module as default` | The entry has no default export, or `key` in `defineModule` differs from the key in `plugin.toml`. |
| `Failed to resolve module specifier "..."` | The bundle imports a package that is not in the import map and was not bundled. Bundle it, or use one of the shared modules. |
| `Invalid hook call` or context errors | The bundle contains its own copy of React, React Router or TanStack Query. Mark them as external. |
| `record resource not registered: <type>` | A record component was rendered for a type that is not registered yet. Wait for `useCustomObjects()` to load before rendering, or check the record type spelling (`<plugin key>.<entity>`). |
| Your screens do not change after an upgrade | Chunks or assets without content hashes were served from the browser cache (they are cached for a year). Add hashes to every file name except the entry. |
| The whole application is blank after install | An exception was thrown inside `register`. Read the console, then disable or uninstall the plugin through the API (`POST /api/v1/wasm-runtime/<key>/disable`). |
| `[aphel] "<id>" is overridden by ...` (development builds) | Several modules override the same component. The last one wins. |

### The network panel

- `GET /api/v1/public/wasm-runtime` lists the bundles that will load, with `entry` and `style` URLs. If your plugin is missing, it is disabled, it has no `[web]`, or it is an interface plugin that is not the selected one. Try `?ui=<key>` to see the list for that interface.
- Open the `entry` URL directly. It must return JavaScript. A 404 means the file is not under `web/` in the package or the plugin is disabled.
- `GET /api/v1/public/ui/<key>.css` shows the stylesheet of an interface plugin. A 404 means it is disabled or has no `[ui]`. An empty body means no tokens apply.
- Failed API calls carry the error envelope `{ "error": { "code", "message", "details" } }`. In code, catch `ApiError` (use `isApiError(error)`) and read `error.status` and `error.code`.

### Errors and codes

| Code | Status | Meaning and fix |
| --- | --- | --- |
| `plugin_package_invalid` | 422 | The zip is unreadable, too large, has too many files, unsafe paths, a missing `plugin.toml`, a missing web entry or a public app without `index.html`. The message names the problem. |
| `plugin_manifest_invalid` | 422 | `plugin.toml` does not parse or validate. `details.fields` lists each path and message, such as `ui.tokens.light.background` or `requires_aphel`. |
| `plugin_backend_missing` | 422 | The manifest declares callbacks but the package has no `backend.wasm`. |
| `plugin_backend_invalid` | 422 | `backend.wasm` is not a valid plugin component. |
| `plugin_web_unacknowledged` | 409 | The package has `[web]` and `acknowledge_web_version` is missing or differs from `version`. Also returned when enabling. |
| `plugin_conflicts_unacknowledged` | 409 | Overlap with another plugin. Review `conflicts` in the dry run and resend with `acknowledge_conflicts=true`. |
| `plugin_module_conflict_blocked` | 409 | The plugin would replace a built-in module that other plugins hold. See `module_conflicts`. |
| `plugin_has_dependents` | 409 | Other enabled plugins depend on this one. |
| `plugin_object_conflict`, `plugin_field_conflict`, `permission_already_registered` | 409 | A custom object, field or permission with the same key already exists. |
| `plugin_role_preset_unknown` | validation | `role_presets` names a preset the manifest does not declare. |
| `plugin_not_found` | 404 | The plugin is not installed, or is disabled for the change feed and for the public app. |
| `permission_denied` | 403 | The token or user lacks `system.manage`. |
| `csrf_header_missing` | 403 | A session request that changes data lacks `X-Requested-With: aphel`. Tokens do not need it. |
| `plugin_failed` | 503 | The backend of the plugin stopped loading. See `backend_error`. |

### `backend_error`

`GET /api/v1/wasm-runtime` returns `backend_error` for each plugin. It is null when the backend loads. When it is set, writes to the record types that the plugin hooks fail with `plugin_failed` until you reinstall a working package. Web-only plugins never have a backend error.

## Following changes in real time

`useChangeFeed` subscribes to the change feed of a plugin, so a screen can refresh when its data changes:

```tsx
import { useQueryClient } from '@tanstack/react-query'
import { recordsKeys, useChangeFeed } from '@aphel/web-sdk'

export function useLiveDeals() {
  const queryClient = useQueryClient()
  useChangeFeed(
    'deal_watch',
    (change) => {
      if (change.record_type) void queryClient.invalidateQueries({ queryKey: recordsKeys.list(change.record_type) })
    },
    { onReset: () => void queryClient.invalidateQueries() },
  )
}
```

Signature: `useChangeFeed(pluginKey, onChange, { enabled = true, since = 0, onReset }?)`. `onChange` receives `{ seq, event_type, record_type, record_id, fields, occurred_at }`. `fields` holds the changed field names, never their values, so read the record to get them.

- The feed contains the creates, updates, deletes, archives and restores of the entities of that plugin and every `plugin.<key>.*` event, in commit order, with an increasing `seq`.
- Entries for record types that the user cannot read are skipped.
- The feed keeps 24 hours. If entries after `since` were pruned, the server sends `reset` and the hook calls `onReset`, where you should reload your state.
- The stream is Server-Sent Events. The server closes it after 5 minutes and the browser reconnects with `Last-Event-ID`, which the hook handles.
- A plugin that is not installed or is disabled returns 404 `plugin_not_found`.
- The same data is available as polling: `GET /api/v1/wasm-runtime/<key>/changes?since=<seq>&limit=<n>` returns `{ data, next, reset }`, with at most 500 entries per page.

The hook calls the latest `onChange` you passed, so you do not need to memoize it.

## Backend logs

Messages written with `log` in the backend and the server errors of plugin calls appear in the Aphel server logs (`RUST_LOG` controls the level). See [Operations](/_aphel/docs/admin/operations/) for how to read them, and [Host API](/_aphel/docs/plugins/host-api/) for `log`.

## Related pages

- [Packaging](/_aphel/docs/plugins/packaging/)
- [Web bundle](/_aphel/docs/plugins/web-bundle/)
- [Errors](/_aphel/docs/reference/errors/)
