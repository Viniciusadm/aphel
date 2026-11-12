---
title: Plugin packs
description: Ship several plugins as one .aphelpack file, installed in dependency order with one consent, optional settings and suggested roles.
sidebar:
  order: 56
---

A plugin pack bundles several plugins in one file, like a modpack. The administrator sends one `.aphelpack`, reviews one consent screen and Aphel installs every plugin in the order their dependencies need. Use a pack when a system is made of plugins that work together, such as a point of sale with its catalog, payments, fiscal documents and printing.

After installing, the plugins stay grouped under the pack name on the plugins screen. Each plugin can still be enabled, disabled, configured or uninstalled on its own.

## Source layout

A pack is described by a `pack.toml` next to the plugin folders it points to:

```toml
key = "ludovico"
name = "Ludovico: day pass e lanchonete"
version = "1.0.0"
description = "Register, menu, day pass with gate, fiscal receipt and printing."
role_presets = ["caixa", "portaria", "gerente_loja"]

[[plugins]]
path = "../pos"
[plugins.settings]
accept_terminal = true

[[plugins]]
path = "../retail_catalog"
```

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `key` | string | yes | Lowercase key of up to 48 characters (letters, digits, `_`, `-`). Identifies the pack across versions. |
| `name` | string | yes | Name shown on the plugins screen, up to 120 characters. |
| `version` | string | yes | Semantic version of the pack. |
| `description` | string | no | Shown on the consent screen. |
| `role_presets` | array of strings | no | Role presets that come checked on the consent screen. Each must be declared by a plugin of the pack. |
| `plugins[].path` | string | yes in the source | Folder of the plugin, relative to `pack.toml`. |
| `plugins[].settings` | table | no | Setting values for that plugin. Secrets are refused. |
| `requires_aphel` | string | no | Semantic version requirement for Aphel, such as `">=0.1.0"`. |
| `requires_modules` | array of strings | no | Native modules the pack needs, such as `["site", "publication"]`. |
| `provision` | array of tables | no | Records and settings the pack creates after installing, see [Provisioning](#provisioning). |

The order of `[[plugins]]` does not matter. Aphel computes it.

## Building

```bash
aphel-plugin bundle plugins/ludovico-pack
```

`bundle` runs `build` and `pack` in every plugin folder and writes `<key>-<version>.aphelpack` next to `pack.toml` (or to `-o file.aphelpack`). The file is a zip with the final `pack.toml`, where each `path` became `file = "plugins/<key>-<version>.aphelplugin"`, and the plugin packages under `plugins/`.

## Install order

A plugin comes after another plugin of the pack when it:

- calls one of its operations (`operations`);
- listens to one of its events (`events = ["plugin.<key>.<name>"]`);
- requires a contract it provides (`[uses] required`);
- has a `record` field or a reference that targets one of its entities;
- picks a setting from one of its entities (`options_from`).

Plugins with no dependency between them are ordered by key. Two plugins that depend on each other fail with 422 `plugin_pack_dependency_cycle`. An operation that neither Aphel nor the pack offers fails with 422 `plugin_pack_missing_dependency`.

## What the administrator sees

Settings → Integrations → Plugins accepts `.aphelplugin` and `.aphelpack` files on the same button. For a pack the consent screen shows:

- every plugin in install order, marked **New**, **Upgrade** or **No change**;
- what each plugin asks for, the same list a single plugin shows;
- the settings the pack fills in;
- one admission for the integrated frontend code of the whole pack;
- one role checklist with the roles of every plugin, the pack's `role_presets` already checked.

Every plugin is validated before anything is installed. If one of them has a problem, the screen points to it and the install button stays disabled.

## Settings

Pack settings only fill settings that have no value yet. A value the administrator changed on the instance is never overwritten, not even by a newer version of the pack.

## Requirements

`requires_aphel` is checked before anything is installed: an older Aphel fails with 422 `plugin_pack_aphel_incompatible`. Each module in `requires_modules` must be part of the Aphel build, or the pack fails with 422 `plugin_pack_module_unavailable`. A required module that is built in but disabled is listed on the consent screen and enabled by the install, before the plugins. When a plugin of the pack declares entities, the module that administers objects (`objects`) is required too, even if `requires_modules` does not list it.

## Provisioning

A pack can create what its plugins need to be useful right away, such as a first blog and a menu link to it. Each `[[provision]]` describes one resource:

```toml
[[provision]]
key = "blog.collection"
kind = "record"
target = "content.collection"
mode = "seed"
unique = "path"
path_conflict = "suffix"
data = { name = "Blog", path = "blog", description = "News and articles." }

[[provision]]
key = "blog.menu"
kind = "site.link"
depends_on = ["blog.collection"]
data = { id = "blog", label = "Blog", url = { provision = "blog.collection", field = "path", prefix = "/" } }
```

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `key` | string | yes | Unique key of the resource inside the pack. Aphel remembers what it created under this key. |
| `kind` | string | yes | What to create. `record` (module `wasm_runtime`) creates a record of `target`; `site.link` (module `site`) adds a link to the site menu; `tagging.vocabulary` (module `tagging`) creates a vocabulary from `key`, `name`, `hierarchical` and `public`. |
| `target` | string | for `record` | Entity key, such as `content.collection`. |
| `mode` | `seed` or `managed` | no | `seed` (default) creates the resource once and never touches it again. `managed` updates it when a newer pack changes `data`, unless an administrator changed or removed it. |
| `data` | table | yes | Field values. A value `{ provision = "<key>", field = "<field>", prefix = "..." }` takes a field of a resource created earlier in the pack, with an optional prefix. |
| `depends_on` | array of strings | no | Resources that must be created first. Referenced resources must be listed here. |
| `unique` | string | no | Field that must be unique, such as an address. |
| `path_conflict` | `fail` or `suffix` | no | With `suffix`, a taken `unique` value gets `-2`, `-3` and so on instead of failing. |

Resources are created after the plugins, in dependency order. The result of each one is reported with a status:

| Status | Meaning |
| --- | --- |
| `created` | Created by this install. |
| `updated` | A `managed` resource was updated to the new `data`. |
| `kept` | Already created by an earlier install and left as it is. |
| `user_changed` | An administrator changed it, so the pack left it alone. |
| `removed` | An administrator deleted it, so the pack did not create it again. |
| `failed` | It could not be created; `error` says why. The install answers 207. |
| `skipped` | A resource it depends on failed. |

Sending the same pack again creates nothing twice. A resource that failed is retried on the next install. A `kind` that no enabled module provides fails with `plugin_pack_provisioner_unknown`, and a reference to a resource the pack does not create, or to a field it does not have, fails with `plugin_pack_provision_reference`.

## Upgrades

Send a newer version of the pack. Plugins whose package is identical to the installed one (same checksum) are skipped; the others are upgraded in order. A plugin installed by itself over a pack plugin leaves the pack.

If a plugin fails while installing, the following plugins are skipped and the screen lists what was installed. Fix the problem and send the pack again: the plugins that already went in are now unchanged and skipped, so the install continues from where it stopped.

## API

| Request | What it does |
| --- | --- |
| `POST /api/v1/wasm-runtime/packs?dry_run=true` | Multipart field `package`. Returns the pack, its plugins in order with `action`, `consent`, `error` and `settings`, the modules it will enable (`modules_to_enable`) and the planned resources (`provisions`). Nothing is installed. |
| `POST /api/v1/wasm-runtime/packs` | Installs. `acknowledge_web_version=<pack version>` admits the frontend code of every plugin of the pack, `acknowledge_conflicts=true` acknowledges overlaps with plugins outside the pack and `role_presets` (comma separated) picks the roles. Returns `modules_enabled` and the status of every resource in `provisions`. 201 when everything went in, 207 when a plugin or a resource failed. |
| `GET /api/v1/wasm-runtime/packs` | Installed packs and the plugins that belong to each. |

All of them need `system.manage`. Installing publishes `plugin.pack_installed` and one `plugin.installed` per plugin.

## Related pages

- [Packaging and installing](/_aphel/docs/plugins/packaging/)
- [Lifecycle](/_aphel/docs/plugins/lifecycle/)
- [Settings and secrets](/_aphel/docs/plugins/settings-and-secrets/)
