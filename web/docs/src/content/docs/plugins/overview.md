---
title: Plugin overview
description: What a runtime plugin is, what it can extend, how it is packaged and how it fits with entities, records and permissions.
sidebar:
  order: 1
---

A runtime plugin is a package that adds behavior to a running Aphel installation without rebuilding or restarting it. An administrator uploads a `.aphelplugin` file, reviews what it asks for in a consent dialog, and the plugin starts working: new entities and fields appear, hooks run when records change, routes answer requests, schedules fire and a web bundle can add screens.

Runtime plugins are different from the built-in modules that are compiled into the Aphel binary. Built-in modules ship with Aphel and are chosen when the binary is built. Runtime plugins are installed afterwards, by anyone with the `system.manage` permission, and they can only reach what the plugin API exposes. They never run SQL, never touch the file system and never call the network except hosts they declare.

## What a plugin can do

| You want to | Use | Page |
| --- | --- | --- |
| Store your own data with real columns and indexes | `[[entities]]` with `storage = "managed"` | [Entities and fields](/docs/plugins/entities-and-fields/) |
| Add fields to Aphel's own record types | `[[fields]]` | [Entities and fields](/docs/plugins/entities-and-fields/) |
| Add permissions and suggest roles | `[[permissions]]`, `[[role_presets]]`, `requires` | [Permissions and roles](/docs/plugins/permissions-and-roles/) |
| Validate, reject or rewrite a record write | `[[hooks.subscribe]]` | [Hooks](/docs/plugins/hooks/) |
| React to something that already happened | `events` | [Events](/docs/plugins/events/) |
| Expose an HTTP endpoint, private or public | `[[routes]]` | [Routes](/docs/plugins/routes/) |
| Run work on a timer | `[[schedules]]` | [Schedules](/docs/plugins/schedules/) |
| Let admins configure the plugin, store API keys | `[[settings]]` | [Settings and secrets](/docs/plugins/settings-and-secrets/) |
| Add automation actions and triggers, notifications, dashboard widgets | `[[actions]]`, `[[emits]]`, `[[notifications]]`, `[[widgets]]` | [Automation and widgets](/docs/plugins/automation-and-widgets/) |
| Offer operations to other plugins or replace a built-in module | `[provides]`, `[uses]`, `[[contribute]]` | [Automation and widgets](/docs/plugins/automation-and-widgets/), [Lifecycle](/docs/plugins/lifecycle/) |
| Handle privacy requests for your data | `[privacy]` | [Automation and widgets](/docs/plugins/automation-and-widgets/) |
| Add screens in the browser | `[web]` | [Web bundle](/docs/plugins/web-bundle/) |
| Replace the whole interface | `[ui]` | [UI plugins](/docs/plugins/ui-plugins/) |

Everything the backend does goes through the host API: records, key-value storage, outbound HTTP, notifications, events and so on. See [Host API](/docs/plugins/host-api/).

## Package layout

A package is a zip file with the extension `.aphelplugin`.

| Path | Required | Content |
| --- | --- | --- |
| `plugin.toml` | yes | The manifest. See [Manifest](/docs/plugins/manifest/). |
| `backend.wasm` | when the manifest declares any runtime callback | A WebAssembly component of the `aphel:plugin` world, built with the [JavaScript SDK](/docs/plugins/js-sdk/) or the [Rust SDK](/docs/plugins/rust-sdk/). |
| `web/...` | no | The browser bundle. `web.entry` (default `web/index.js`), `web.style` and the optional `web.public` application must exist inside the package. |

A manifest needs a backend as soon as it declares hook subscriptions, `events`, `routes`, `schedules`, `[[actions]]`, `[[widgets]]` or `[privacy] subject_data = true`. Without a backend the installer answers `plugin_backend_missing`. A plugin made only of entities, fields, permissions and settings needs no backend.

The `aphel-plugin` command line tool creates, builds and packs these files for you. See [Your first plugin](/docs/plugins/first-plugin/) and [Packaging](/docs/plugins/packaging/).

## Package limits

| Limit | Value | Error |
| --- | --- | --- |
| Package size and unpacked size | 50 MB each | `plugin_package_invalid` |
| Files in the package | 500 | `plugin_package_invalid` |
| Path length | 255 characters; no empty segments, no `.` or `..`, no leading `/`, no backslash | `plugin_package_invalid` |
| Not a zip file, unreadable entry | | `plugin_package_invalid` |
| No `plugin.toml` or not valid UTF-8 | | `plugin_manifest_invalid` |
| `backend.wasm` is not a component of the `aphel:plugin` world | | `plugin_backend_invalid` |
| `web.entry` or `web.public` index missing | | `plugin_package_invalid` |

## How a call runs

Every callback runs in a fresh WebAssembly instance with an empty sandbox: no file system, no environment, no sockets. An instance is limited to 64 MB of memory and a time budget that depends on the entry point.

| Entry point | Time limit |
| --- | --- |
| Hook (including `record.committed`) | 2 seconds |
| Route | 10 seconds |
| Event | 30 seconds |
| Schedule | 60 seconds |
| Automation action, operation, contribution | 30 seconds |
| Widget | 5 seconds |
| Privacy request | 60 seconds |

A callback that exceeds its limit, traps or cannot be loaded fails with `503 plugin_failed`. See [Security model](/docs/plugins/security-model/) and [Lifecycle](/docs/plugins/lifecycle/).

## Trust model in short

- The administrator who installs a plugin consents to everything the consent dialog lists. The plugin can use only permissions it declares in `requires`, `optional_requires`, `[[permissions]]` or through its own entities. These are its capabilities.
- Every call runs with the intersection of the acting user's permissions and the plugin's capabilities. A plugin never gains power the actor does not have, and never uses power it did not declare.
- Events, schedules and public routes have no signed-in user. They run as the plugin itself, with the permissions of the user who installed it, limited to the plugin's capabilities.
- A plugin web bundle is not isolated. It runs in the browser with the signed-in user's session. Installing a plugin with a web bundle needs an explicit acknowledgement for that version.

The full description is in [Security model](/docs/plugins/security-model/).

## How plugins fit with records and permissions

Plugins do not have a private database. They work with the same records, permissions and events as the rest of Aphel.

- A plugin entity `book` in plugin `library` is the record type `library.book`. It gets the permissions `library.book.read`, `.create`, `.update`, `.delete`, `.archive` and `.export`, and shows up in the generic record API at `/api/v1/entities/library.book/records` like any other record type.
- Record writes, including those made by plugin code, go through the same pipeline as writes from the UI: authorization, hooks, validation, audit and events. See [Hooks](/docs/plugins/hooks/).
- Plugin fields added to built-in types are ordinary custom fields owned by the plugin. See [Entities and fields](/docs/plugins/entities-and-fields/).
- Roles are not owned by plugins. A plugin declares permissions and may offer role presets, which the administrator selects at install time. See [Permissions and roles](/docs/plugins/permissions-and-roles/).

## Where to start

1. Build the smallest useful plugin in [Your first plugin](/docs/plugins/first-plugin/).
2. Keep the [Manifest](/docs/plugins/manifest/) reference open while you design.
3. Read [Hooks](/docs/plugins/hooks/) and [Host API](/docs/plugins/host-api/) before writing backend code, in particular the rules about side effects.
4. Test with the guidance in [Testing and debugging](/docs/plugins/testing-and-debugging/).
