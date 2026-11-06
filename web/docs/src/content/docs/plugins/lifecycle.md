---
title: Lifecycle
description: How plugins are installed, consented to, upgraded, enabled, disabled and uninstalled, and how conflicts and failures are handled.
sidebar:
  order: 15
---

This page follows a plugin from the first upload to its removal, with the endpoints, the checks the installer runs and the errors you can get. The endpoints need the `system.manage` permission, which the `admin` role has.

| Endpoint | Purpose |
| --- | --- |
| `POST /api/v1/wasm-runtime` | Install or upgrade. Multipart form, field `package`. |
| `POST /api/v1/wasm-runtime?dry_run=true` | Check a package and return what it asks for. Installs nothing. |
| `GET /api/v1/wasm-runtime` | List installed plugins with what they declare. |
| `POST /api/v1/wasm-runtime/{key}/enable` | Enable. |
| `POST /api/v1/wasm-runtime/{key}/disable` | Disable. |
| `DELETE /api/v1/wasm-runtime/{key}` | Uninstall (`?purge=true` deletes data). |
| `GET`, `PUT /api/v1/wasm-runtime/{key}/settings` | Settings. See [Settings and secrets](/docs/plugins/settings-and-secrets/). |
| `POST /api/v1/wasm-runtime/{key}/grants` | Reapply grants. See [Permissions and roles](/docs/plugins/permissions-and-roles/). |
| `GET /api/v1/wasm-runtime/host-catalog` | Hooks and operations this Aphel offers. |
| `GET /api/v1/wasm-runtime/readiness` | Catalog state: `{ state, version, backend_errors }`. |

Query parameters of install and enable: `acknowledge_conflicts=true` and `acknowledge_web_version=<manifest version>`. The install form also takes `role_presets` (comma-separated role preset keys).

Each change publishes a domain event (`plugin.installed`, `plugin.uninstalled`, `plugin.enabled`, `plugin.disabled`, `plugin.settings_updated`, `plugin.grants_reapplied`) and an audit entry in the same transaction.

## Dry run and consent

A dry run runs every check of a real install and answers `200` instead of installing. The interface always starts with a dry run and shows the result in the consent dialog; with the API you call it yourself.

```bash
curl -sS -X POST "$APHEL_URL/api/v1/wasm-runtime?dry_run=true" \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -F "package=@library-0.1.0.aphelplugin"
```

The response lists what the administrator is consenting to.

| Field | Content |
| --- | --- |
| `key`, `name`, `version`, `description`, `installed_version` | Identity; `installed_version` is set when this is an upgrade. |
| `entities`, `extended_types` | Record types the plugin creates and record types it adds fields to. |
| `hooks`, `events`, `routes`, `public_routes` | Hook subscriptions (with record types), consumed events, private and public routes. A public web app adds `GET /p/<key>/`. |
| `requires`, `optional_requires` | Permissions it uses. |
| `permissions` | Permissions it creates, with `grant_to`. |
| `allowed_hosts` | Hosts it may call. |
| `schedules`, `secrets`, `actions`, `emits`, `notifications`, `widgets`, `operations` | Features it adds. |
| `role_presets`, `entity_scopes`, `unique_fields` | Roles it offers, default grant scopes, unique fields. |
| `menu`, `replaces`, `ui`, `has_web` | Navigation options, built-in modules it would replace, interface name, whether it ships a web bundle. |
| `grants` | Per role, the permissions expected and already granted. |
| `conflicts` | Overlaps with other installed plugins: `{ kind, target, other_plugin, other_name }`, `kind` being `hook_overlap` or `role_preset_key`. |
| `module_conflicts` | Built-in modules that would be disabled: `{ module, name, enabled, blocked_by }`. |

## Install

The installer runs these steps in order and stops at the first failure.

| Step | Checks | Errors |
| --- | --- | --- |
| Package | Size, file count, paths, zip, web entry | `422 plugin_package_invalid` |
| Manifest | Every rule in [Manifest](/docs/plugins/manifest/) | `422 plugin_manifest_invalid`, `422 plugin_backend_missing` |
| Backend | The component compiles and has the `aphel:plugin` world | `422 plugin_backend_invalid` |
| Presets | Every key in `role_presets` is a preset of the package | `422 plugin_role_preset_unknown` |
| Dependents | For an enabled plugin being replaced: other plugins must not lose a contract or operation they use | `409 plugin_has_dependents` |
| Schema | Entity storage and indexes; required contracts exist; required permissions belong to enabled modules; reference permissions; field type changes; key conflicts; unique values | `422 plugin_contract_unavailable`, `422 plugin_dependency_unavailable`, `422 plugin_reference_permission_required`, `409 plugin_field_type_changed`, `409 plugin_object_conflict`, `409 plugin_field_conflict`, `409 plugin_field_unique_conflict` |
| Conflicts and modules | Overlaps with other plugins, modules to disable | listed in the dry run |

A dry run stops here. A real install continues:

| Step | Checks | Errors |
| --- | --- | --- |
| Web admission | A package with `[web]` needs `acknowledge_web_version` equal to the manifest version | `409 plugin_web_unacknowledged` |
| Conflicts | If there are conflicts, `acknowledge_conflicts=true` | `409 plugin_conflicts_unacknowledged` (`details.conflicts` lists them) |
| Modules | Modules the plugin replaces must be free to be disabled | `409 plugin_module_conflict_blocked` |
| Apply | Creates objects and tables, writes fields, stores the package, registers permissions (`409 permission_already_registered` if one exists), reconciles grants, selected role presets, publishes the event | |

Applying is not one transaction. The schema is applied first and the package is stored afterwards; if a later step fails, fix the cause and install again. The installer user is recorded as the owner of the plugin (`installed_by`): events, schedules and public routes run with that user's permissions.

The answer is `201` with the plugin. The catalog is invalidated; every Aphel process notices within a few seconds and compiles the backend on first use.

Installing a package with the key of an installed plugin is an upgrade.

## Upgrade

The installer does not compare versions, so any package with the same key replaces the installed one, including an older version. The new version is checked as above, and then reconciled with the installed state.

| Area | Result |
| --- | --- |
| Settings values | Kept, including values of settings the new version dropped. |
| Key-value data, secrets | Kept. |
| Enabled state | Kept. A disabled plugin stays disabled. |
| Owner (`installed_by`) | Becomes the user who installed the upgrade. |
| Entities, fields, options | Created, updated or deactivated as in [Entities and fields](/docs/plugins/entities-and-fields/#upgrades-deactivation-and-type-changes). |
| Permissions | Re-registered. Removed declarations disappear from the catalog. |
| Grants | Only added. Existing grants are never removed. |
| Web bundle | Replaced. The browser loads the new files at its next page load. |

Refusals specific to upgrades: `409 plugin_field_type_changed` when a field changes type, `409 plugin_field_unique_conflict` when a new `unique` clashes with data, `409 plugin_has_dependents` when other plugins depend on a contract or operation that the new version no longer provides, and a configuration error when the entity `storage` changes.

## Enable and disable

**Disable** (`POST /{key}/disable`) turns the plugin off without deleting anything. Its entities and fields are hidden and its permissions become inactive. Hooks, events, routes, schedules, actions, widgets and the web bundle stop. Modules it replaced come back. Records, settings and key-value data are kept. Disabling is refused with `409 plugin_has_dependents` when an enabled plugin needs it.

Hooks of a disabled plugin no longer run, so record types that it validated accept writes it would have rejected.

**Enable** (`POST /{key}/enable`) repeats the checks that can change over time: web admission when the plugin has a bundle (`acknowledge_web_version`), required contracts and modules (`plugin_contract_unavailable`, `plugin_dependency_unavailable`), conflicts with other enabled plugins (`acknowledge_conflicts=true`) and modules to replace. Then the entities, fields and permissions come back.

## Uninstall and purge

`DELETE /api/v1/wasm-runtime/{key}` removes the plugin code, web bundle, key-value data, declared permissions and secrets, and clears the plugin as a chosen interface. It answers `204`. It is refused with `409 plugin_has_dependents` while other plugins depend on it.

| | Without `purge` | With `purge=true` |
| --- | --- | --- |
| Entities | Deactivated, with records | Deleted with all records, attachments, shares and field values; managed tables are dropped |
| Fields added to other record types | Deactivated | Deleted |
| Role grants for the plugin's entities | Kept | Deleted |
| Reinstalling | Brings entities and records back | Starts empty |

Purging cannot be undone.

## Conflicts and acknowledgement

Two kinds of overlap between enabled plugins need the administrator's explicit acknowledgement:

- `hook_overlap`: both subscribe to `record.saving`, `record.validating`, `record.deleting` or `record.archiving` for the same record type. Entities owned by either plugin are exempt.
- `role_preset_key`: both offer a preset with the same role key.

Without `acknowledge_conflicts=true` the install or enable answers `409 plugin_conflicts_unacknowledged` and `details.conflicts` holds the list. The consent dialog shows the same list with a checkbox.

## Replacing a built-in module

A plugin that lists a contract of a built-in module in `provides.contracts` replaces that module while the plugin is enabled.

```toml
[provides]
contracts = ["name@1"]

[aliases]
"/some-module" = "/compat/some-module"
```

- The replaced modules are the built-in modules whose declared contract requirement appears in `provides.contracts`. The dry run lists them in `replaces` and `module_conflicts`.
- On install or enable the module is disabled and held. The installer needs the `settings.manage` permission for this. A module that is required, or that other enabled modules or plugins depend on, cannot be replaced: `409 plugin_module_conflict_blocked`, with `blocked_by` in the dry run.
- When the plugin is disabled or uninstalled the module is released, and enabled again if it was enabled before and no other plugin still holds it.
- `[aliases]` maps API paths of the replaced module to routes of the plugin: a request to `/api/v1/some-module/...` is answered by the private route `/compat/some-module/...`, which must be declared in `[[routes]]` with a pattern that matches the full remaining path. The route runs with the caller's permissions limited to the plugin's capabilities. Aliases are only allowed together with `provides.contracts` that replace a module.
- A plugin that holds a module may provide operations with the keys of that module's operations. Without holding it, a key owned by a built-in module is refused (`is already provided by a built-in module`).

## Durable execution

Background work of plugins is stored in the database and run by workers:

- Event deliveries, schedule slots, committed hooks and deferred effects each have an execution record with status, last error and the call chain. A record is created once per source (event id, schedule slot, effect id), which is what makes retries safe.
- Jobs retry with a growing delay: five attempts by default, three for schedule slots. A completed execution is never run again.
- Deferred effects carry the authority captured when they were created and are re-resolved when they run. If the plugin was disabled or removed meanwhile, they fail with `503 plugin_disabled`.
- Plugin jobs need a worker. Without one nothing runs.

See [Events](/docs/plugins/events/) and [Schedules](/docs/plugins/schedules/).

## Failure modes

| What happens | What the plugin or the user sees |
| --- | --- |
| The backend cannot be loaded (missing `backend.wasm`, or a component that no longer compiles or instantiates) | The plugin list shows `backend_error`. Calls fail with `503 plugin_failed`. The failure is closed: writes to the record types the plugin hooks are blocked until it is reinstalled. |
| A call times out, traps, runs out of memory | `503 plugin_failed`. The detail is in the server log. |
| The installer is no longer an active member | Events, schedules, privacy requests and public routes fail with `503 plugin_owner_inactive`. An administrator must reinstall the plugin so it gets a new owner. |
| A plugin is disabled while jobs are queued | Jobs fail with `503 plugin_disabled` and retry until their attempts end. |
| The plugin catalog changes during a request | `503 plugin_catalog_changed`. Retry the request. |
| A plugin is requested that does not exist or is disabled | `404 plugin_not_found`; routes answer `404 plugin_route_not_found` for unknown paths. |
| A step in an automation uses an action of a plugin that is gone | `503 plugin_unavailable` |
| A hook returns an error | The caller gets the error and the write is rolled back. See [Hooks](/docs/plugins/hooks/). |

The Plugins page shows `backend_error` and the grant status per plugin. `GET /api/v1/wasm-runtime/readiness` reports `{ state, version, backend_errors }`; it answers `loading` while the catalog is being read and `503` when the catalog is unavailable.
