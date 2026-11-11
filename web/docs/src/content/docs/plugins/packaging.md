---
title: Packaging and installing
description: Build a .aphelplugin package with the aphel-plugin CLI, then install, upgrade and remove it from the interface or the API.
sidebar:
  order: 55
---

A plugin is distributed as a single `.aphelplugin` file, which is a zip archive. The `aphel-plugin` command line tool creates project skeletons, builds the backend and the web bundle, and packs the result.

## The `aphel-plugin` CLI

Build it from the Aphel source tree:

```bash
cd sdk/cli
cargo build --release
```

The binary is `sdk/cli/target/release/aphel-plugin`. It needs `cargo` and the `wasm32-wasip2` target for Rust backends, and `npm` for JavaScript backends and web bundles.

```text
aphel-plugin new <key> [--js] [--sdk <path to aphel-plugin-sdk>]
aphel-plugin build [dir]
aphel-plugin pack [dir] [-o file.aphelplugin]
aphel-plugin bundle [dir with pack.toml] [-o file.aphelpack]
```

`bundle` builds a [plugin pack](/_aphel/docs/plugins/packs/) from a `pack.toml`. The `aphel` binary embeds the same commands as `aphel plugin new|build|pack|bundle`, with identical arguments, so a separate build of the CLI is optional. `dir` defaults to the current directory. Unknown commands print the usage and exit with an error. There is no `--help` flag.

### `new`

| Argument | Meaning |
| --- | --- |
| `<key>` | The plugin key. Used in `plugin.toml` and for the directory name `aphel-<key>` (underscores become dashes), created in the current directory. The command fails if the directory exists. The key is not validated here. The installer validates it. |
| `--js` | Create a JavaScript backend (`package.json`, `src/plugin.js`, `.gitignore`) instead of a Rust one (`Cargo.toml`, `src/lib.rs`). |
| `--sdk <path>` | Path to the SDK the project depends on. Defaults to `../aphel/sdk/rust/aphel-plugin-sdk` (Rust) or `../aphel/sdk/js/aphel-plugin-sdk` (JavaScript). |

The generated `plugin.toml` declares the key, `name` equal to the key, `version = "0.1.0"`, `api = 1`, `requires_aphel = ">=0.1.0"`, `[menu] flat = true` and one `GET /ping` route. `new` creates only a backend. Add a `web/` directory yourself for the browser side.

### `build`

1. Reads `plugin.toml` of `dir` (it must exist and be valid TOML).
2. Backend, in this order:
   - if `package.json` defines a `build:backend` script, it runs `npm install` (when `node_modules` is missing) and `npm run build:backend`;
   - otherwise, if `Cargo.toml` exists, it runs `cargo build --release --target wasm32-wasip2` and copies `target/wasm32-wasip2/release/<crate name>.wasm` to `backend.wasm`;
   - otherwise there is no backend step.
3. Web: if `web/package.json` defines a `build` script, it runs `npm install` in `web/` (when `node_modules` is missing) and `npm run build`.

A project with only `[ui]` colors, or with prebuilt files, needs no build.

### `pack`

Runs `build`, then writes the package:

- `plugin.toml`
- `backend.wasm` when it exists in `dir`
- every file under `web/dist/`, stored as `web/<path relative to dist>`
- every file under `site/` (site templates, islands and theme assets), stored as is

The output file is `<dir>/<key>-<version>.aphelplugin` by default, using `key` and `version` from `plugin.toml`. `-o file.aphelplugin` chooses another path (relative to the current directory). The CLI prints `packed <path>`.

Because only `web/dist/` is packed, the paths in `[web]` map to your build output: `entry = "web/index.js"` is `web/dist/index.js`, `style = "web/style.css"` is `web/dist/style.css`, and `public = "web/app"` is `web/dist/app/`. A no-build plugin keeps its files directly in `web/dist/`.

## Package format and limits

A package is a zip file with these rules, enforced by the server on every install:

| Rule | Limit |
| --- | --- |
| Compressed size | At most 50 MB. |
| Unpacked size | At most 50 MB in total. |
| Number of files | At most 500. |
| Path | At most 255 characters, relative, no `\`, no empty segment, no `.` or `..` segment. |
| `plugin.toml` | Required, UTF-8, valid manifest. |
| `backend.wasm` | Required when the manifest declares hooks, events, routes, schedules, actions, widgets or subject-data privacy. |
| Web entry | When `[web]` is declared, the file in `entry` must exist. |
| Public app | `public` must start with `web/` and contain `index.html`. |

A violation fails with HTTP 422 and one of `plugin_package_invalid`, `plugin_manifest_invalid`, `plugin_backend_missing` or `plugin_backend_invalid`.

## Compatibility fields

| Field | Meaning |
| --- | --- |
| `api` | The plugin API version. This Aphel supports `1`. Any other value fails on the `api` field. |
| `requires_aphel` | Optional semantic version requirement, such as `">=0.1.0"`, matched against the running Aphel version. A requirement that does not match fails on `requires_aphel` with "requires Aphel ..., this is ...". An invalid requirement fails with "must be a version requirement". |
| `version` | Semantic version of the plugin, for example `1.2.0`. |

The full manifest is described in [Manifest](/_aphel/docs/plugins/manifest/).

## Installing from the interface

Open Integrations, then Plugins (`/integrations/wasm-runtime`, needs `system.manage`) and choose Install plugin. The interface:

1. Uploads the file for a dry run and opens the review dialog with what the plugin declares, the permissions it asks for, overlaps with other plugins and modules it replaces, and the role presets it offers (unchecked).
2. For a plugin with web code, asks you to confirm that you trust this version to run as application code.
3. Installs when you confirm, and reloads the application.

## Installing through the API

Send the package as multipart form data to `POST /api/v1/wasm-runtime`, with a session or an API token whose scopes include `system.manage`. Requests with a bearer token do not need the `X-Requested-With` header.

| Item | Where | Meaning |
| --- | --- | --- |
| `package` | multipart field | Required. The `.aphelplugin` file. |
| `role_presets` | multipart field | Optional. Comma separated keys of the manifest `[[role_presets]]` to create. An unknown key fails with `plugin_role_preset_unknown`. |
| `dry_run` | query | `true` validates everything and answers 200 with the review data, installing nothing. Needs no web acknowledgement. |
| `acknowledge_web_version` | query | Must equal the manifest `version` when the package declares `[web]`. Required on every install and upgrade, including upgrades of a disabled plugin. |
| `acknowledge_conflicts` | query | `true` accepts overlaps with other installed plugins. |

Review first, then install:

```bash
curl -s -X POST 'https://crm.example.com/api/v1/wasm-runtime?dry_run=true' \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -F package=@deal_watch-0.1.0.aphelplugin
```

```bash
curl -s -X POST 'https://crm.example.com/api/v1/wasm-runtime?acknowledge_web_version=0.1.0' \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -F package=@deal_watch-0.1.0.aphelplugin
```

The dry run answers `{ "data": { ... } }` with `key`, `name`, `version`, `description`, `installed_version`, the declarations (`entities`, `extended_types`, `hooks`, `events`, `routes`, `public_routes`, `requires`, `optional_requires`, `permissions`, `allowed_hosts`, `schedules`, `secrets`, `actions`, `emits`, `notifications`, `widgets`, `operations`, `role_presets`, `entity_scopes`, `unique_fields`, `menu`, `replaces`, `ui`, `has_web`), `grants`, `conflicts` and `module_conflicts`. A real install answers 201 with the installed plugin, including `enabled` and `backend_error`.

Other endpoints, all needing `system.manage`:

| Request | Effect |
| --- | --- |
| `GET /api/v1/wasm-runtime` | Lists installed plugins. |
| `POST /api/v1/wasm-runtime/{key}/enable` | Enables. Accepts `acknowledge_web_version` and `acknowledge_conflicts`. |
| `POST /api/v1/wasm-runtime/{key}/disable` | Disables without removing anything. Its web bundle and public app stop being served. |
| `DELETE /api/v1/wasm-runtime/{key}?purge=true` | Uninstalls. Without `purge`, entities and fields are only deactivated and come back if the plugin is installed again. |
| `POST /api/v1/wasm-runtime/{key}/grants` | Reapplies the declared grants. |

## Versioning and upgrades

- Install a newer package with the same key to upgrade. The API does not compare versions, so installing the same or an older version also works. Every install needs the web acknowledgement again for the exact version being installed.
- An upgrade replaces the manifest and all package files. It keeps the enabled state (an upgrade of a disabled plugin stays disabled) and the saved settings.
- The cache of the browser is keyed by the package checksum, so users get the new entry file after a reload. Give chunks and assets hashed names. See [Web bundle](/_aphel/docs/plugins/web-bundle/).
- Removing a field, entity or option from the manifest deactivates it on upgrade and keeps its data. See [Lifecycle](/_aphel/docs/plugins/lifecycle/).
- If other enabled plugins depend on yours, a change that would break them fails with 409 `plugin_has_dependents`.

## Related pages

- [Plugin packs](/_aphel/docs/plugins/packs/)
- [Testing and debugging](/_aphel/docs/plugins/testing-and-debugging/)
- [Lifecycle](/_aphel/docs/plugins/lifecycle/)
- [First plugin](/_aphel/docs/plugins/first-plugin/)
