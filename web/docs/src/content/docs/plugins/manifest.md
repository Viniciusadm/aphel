---
title: Manifest reference
description: Every key of plugin.toml, with types, defaults, limits and the errors the installer returns.
sidebar:
  order: 3
---

`plugin.toml` sits at the root of the package. This page lists every key the installer reads. Interface plugins declare an additional `[ui]` table, documented in [UI plugins](/docs/plugins/ui-plugins/).

## How the manifest is checked

The installer parses the file as TOML, then validates it. Both steps run on a dry run (`POST /api/v1/wasm-runtime?dry_run=true`) and on a real install. All problems found by validation are returned together.

| Result | Status | Code | Message and details |
| --- | --- | --- | --- |
| Not valid TOML, wrong value type, unknown key inside a table that rejects unknown keys | 422 | `plugin_manifest_invalid` | `plugin.toml is invalid: <parser message>` |
| Removed key `conflicts_with` | 422 | `plugin_manifest_invalid` | `` `conflicts_with` was removed; provide the contract of the module you replace through provides.contracts `` |
| One or more rules below broken | 422 | `plugin_manifest_invalid` | `plugin.toml is invalid`, with one entry per problem in the field list. Each entry has the path (for example `routes[0].method` or `entities[1].fields[0].type`) and a message. |
| Missing `plugin.toml` or not UTF-8 | 422 | `plugin_manifest_invalid` | `the package has no plugin.toml` / `plugin.toml is not UTF-8` |
| Backend required but absent | 422 | `plugin_backend_missing` | `the plugin declares runtime callbacks but has no backend.wasm` |
| Conflicts with installed state (existing object, field, permission, types) | 409 / 422 | see [Lifecycle](/docs/plugins/lifecycle/) | |

Tables that reject unknown keys are `[hooks]`, `[[hooks.declare]]`, `[menu]`, `[provides]`, `[[provides.operations]]`, `[uses]` and `[[contribute]]`. Every other table silently ignores keys it does not know, so a typo in a key name is not an error there: the setting simply has no effect. Check the spelling against this page.

In TOML, all top-level scalar and array keys must come before the first table header. Put `key`, `name`, `requires`, `events` and so on at the top of the file.

## Top-level keys

| Key | Type | Required | Default | Rules |
| --- | --- | --- | --- | --- |
| `key` | string | yes | | 1 to 32 characters: lowercase letters, digits and `_`; starts with a letter. Not `custom`. Not the key of a module compiled into this Aphel (error `is already used by a built-in module`). If the plugin declares entities, use only letters and underscores, with no `__` and no trailing `_` (see [Entities and fields](/docs/plugins/entities-and-fields/)). |
| `name` | string | yes | | Not blank, at most 120 bytes. |
| `version` | string | yes | | Semantic version, for example `1.2.0`. The installer does not compare versions: installing any version over an installed plugin replaces it. |
| `description` | string | no | none | Free text, shown in the consent dialog and the plugin list. |
| `api` | integer | yes | | Must be `1`. Error: `this Aphel supports plugin API 1`. |
| `requires_aphel` | string | no | none | A semver requirement such as `>=0.1.0`. Errors: `must be a version requirement`, `requires Aphel <req>, this is <version>`. |
| `layer` | `"capability"` or `"domain"` | no | none | Accepted and stored. It has no effect on runtime plugins. |
| `runtime` | `"native"` or `"wasm"` | no | none | Accepted and stored. It has no effect on runtime plugins. |
| `requires` | array of strings | no | `[]` | Permissions the plugin needs. Each must be a permission known to this Aphel, or a permission of an administrator-defined custom object (`<owner>.<object>.<action>`). Error: `is not a known permission`. The permission's owning module must be enabled at install time (`plugin_dependency_unavailable`). |
| `optional_requires` | array of strings | no | `[]` | Permissions the plugin can use when they exist. Each must be a syntactically valid permission (`is not a valid permission`). |
| `settings_permission` | string | no | none | A permission declared in this manifest's `[[permissions]]`. Users holding it may read and change the plugin settings without `system.manage`. Error: `must be a permission declared in permissions`. |
| `events` | array of strings | no | `[]` | Event types the backend receives. Exact names, or a prefix followed by `.*`. The installer does not validate the strings. See [Events](/docs/plugins/events/). |
| `operations` | array of strings | no | `[]` | Host operations the backend may call with `invoke`. At most 100, no duplicates, each offered by this Aphel (`must be a unique operation offered by this Aphel`). See [Host API](/docs/plugins/host-api/). |
| `aliases` | table of string to string | no | `{}` | Maps a built-in API path to a route of this plugin. Both sides must start with `/`. Only allowed together with `provides.contracts` that replaces a built-in module (`aliases need provides.contracts that holds a native module`). See [Lifecycle](/docs/plugins/lifecycle/). |

Capabilities of a plugin are the union of `requires`, `optional_requires`, the keys of `[[permissions]]` and every `<plugin key>.<entity>.<action>` permission of its own entities.

## `[[entities]]`

Declares a record type owned by the plugin. The record type is `<plugin key>.<entity key>`. Details in [Entities and fields](/docs/plugins/entities-and-fields/).

| Key | Type | Required | Default | Rules |
| --- | --- | --- | --- | --- |
| `key` | string | yes | | Lowercase key, at most 32 characters, and `<plugin key>_<entity key>` at most 48 characters. Error: `must be a short lowercase key`. The host also requires the combined object key to be 40 characters or fewer and made of letters and underscores only. |
| `label` | string | yes | | Singular name. |
| `label_plural` | string | yes | | Plural name. |
| `description` | string | no | none | |
| `icon` | string | no | none | Icon name. |
| `storage` | `"dynamic"` or `"managed"` | no | `"dynamic"` | `"table"` is refused (`runtime entities cannot use native table storage`). Changing storage on upgrade is refused. |
| `owner_enabled` | boolean | no | `true` | Whether records have an owner. |
| `permission_scope` | `"own"`, `"team"` or `"organization"` | no | none | Scope of the default grants to the `manager` and `member` roles. Error: `must be own, team or organization`. Without it, `manager` gets `team` and `member` gets `own`. |
| `internal` | boolean | no | `false` | Hides the record type from people-facing lists. |
| `title_field` | string | no | `name`, else the first `text` or `email` field | Field shown as the record's title in lists, links and pickers. Error: `unknown title field`. |
| `indexes` | array of tables | no | `[]` | Managed storage only. Each has `key` (string), `fields` (array of field keys) and `unique` (boolean, default `false`). A unique index only counts records that are not deleted, so a value is free again once its record goes to the trash; a write that breaks it fails with 409 `entity_unique_conflict`, and so does restoring a record whose value was taken meanwhile. With dynamic storage: `entity indexes require managed storage`. |
| `references` | array of tables | no | `[]` | Soft references between records. Each has `field` (a `record` field key of the entity), `target` and `on_delete`. `target` is `{ kind = "entity", entity = "<record type>" }` or `{ kind = "trait", trait_key = "<trait>", type_field = "<field key>" }`. `on_delete` is `"restrict"` (default), `"clear"` or `"cascade"`. A required field cannot use `clear`. A reference to a record type of another owner needs that type's read permission in the plugin's capabilities (`plugin_reference_permission_required`). |
| `traits` | table | no | `{}` | Open map of capability traits merged into the entity. |
| `bulk_fields` | array of strings | no | `[]` | Fields that bulk actions may edit. |
| `fields` | array of tables | no | `[]` | `[[entities.fields]]`, see below. |

## `[[entities.fields]]` and `[[fields]]`

Both tables use the same keys. In `[[fields]]` (top-level) there is one more key, `record_type`, the type to extend.

| Key | Type | Required | Default | Rules |
| --- | --- | --- | --- | --- |
| `record_type` | string | only in `[[fields]]` | | The record type that receives the field. Extending a custom object of another owner is refused (`cannot extend custom objects of other plugins`). |
| `key` | string | yes | | Lowercase key, at most 64 characters. |
| `label` | string | yes | | |
| `type` | string | yes | | One of `text`, `long_text`, `number`, `currency`, `date`, `datetime`, `select`, `multi_select`, `checkbox`, `url`, `email`, `phone`, `user`, `record`, or a field type registered by a module. Error: `is not a known field type`. |
| `required` | boolean | no | `false` | |
| `searchable` | boolean | no | `false` | Included in global search. |
| `unique` | boolean | no | `false` | Only for `text`, `number`, `url`, `email`, `phone`. Error: `is only supported by text, number, url, email and phone fields`. |
| `personal` | boolean | no | `false` | Marks personal data for privacy requests. |
| `sensitive` | boolean | no | `false` | Also implies `personal`. Values are redacted in audit diffs. |
| `min`, `max` | number | no | none | `number` and `currency` only. `min` must not exceed `max`. |
| `decimals` | integer | no | type default | `number` and `currency` only, 0 to 6. |
| `max_length` | integer | no | type default | `text` and `long_text` only. 1 to 1024 for `text`, 1 to 65535 for `long_text`. |
| `pattern` | string | no | none | `text` only. A regular expression of at most 500 characters. |
| `currency` | string | no | none | `currency` only. Three uppercase letters (ISO 4217). |
| `default` | any | no | none | Not allowed for `user` and `record`. |
| `target` | string | for `record` | none | Record type the field points to. Error: `is required for record fields`. Targets outside the plugin need the target's read permission in the capabilities. |
| `read_only` | boolean | no | `false` | Managed entities only. People see the field but cannot change it (422 `field_read_only`); only the plugin's own backend writes it. Use it for totals, statuses and keys that the plugin's operations keep consistent. |
| `options` | array of tables | for `select`, `multi_select` | `[]` | Each has `value` (string), `label` (string) and `color` (optional string). Error: `is required for select fields`. |

A property that the field type does not support is rejected with `is not supported by <type> fields`.

## `[[hooks.subscribe]]` and `[[hooks.declare]]`

`[hooks]` rejects unknown keys.

| Key | Type | Required | Default | Rules |
| --- | --- | --- | --- | --- |
| `hooks.subscribe[].hook` | string | yes | | A hook offered by this Aphel (`GET /api/v1/wasm-runtime/host-catalog`) or declared by an installed plugin. Error: `must be a hook offered by this Aphel`. |
| `hooks.subscribe[].record_types` | array of strings | yes | | At least one (`must list at least one record type`). `"*"` matches every record type. |
| `hooks.subscribe[].priority` | integer | no | `50` | Lower runs first. |
| `hooks.declare[].key` | string | yes | | Dotted key that starts with `<plugin key>.`, each segment a lowercase key, unique within the manifest. |
| `hooks.declare[].description` | string | yes | | 1 to 240 characters. |
| `hooks.declare[].kind` | `"veto"`, `"filter"` or `"action"` | no | `"action"` | |
| `hooks.declare[].record_type` | string | no | `"*"` | |

Declared hooks are added to the host catalog so other plugins can subscribe to them. The plugin interface has no function that fires a declared hook. See [Hooks](/docs/plugins/hooks/).

## `[[routes]]`

| Key | Type | Required | Default | Rules |
| --- | --- | --- | --- | --- |
| `method` | string | yes | | `GET`, `POST`, `PUT`, `PATCH` or `DELETE`, in uppercase. |
| `path` | string | yes | | Starts with `/` and has no `..`. Segments written `{name}` are path parameters. |
| `permission` | string | no | none | A permission known to Aphel, declared by the plugin, or one of its entity permissions. Not allowed on public routes. |
| `public` | boolean | no | `false` | A public route needs no sign-in. Error when combined with `permission`: `public routes cannot require a permission`. |

See [Routes](/docs/plugins/routes/).

## `[[schedules]]`

At most 10.

| Key | Type | Required | Rules |
| --- | --- | --- | --- |
| `name` | string | yes | Unique lowercase key, at most 64 characters. |
| `cron` | string | yes | Cron expression with seconds: `sec min hour day month weekday`. Error: `must be a cron expression with seconds (sec min hour day month weekday)`. |

See [Schedules](/docs/plugins/schedules/).

## `[[settings]]`

| Key | Type | Required | Default | Rules |
| --- | --- | --- | --- | --- |
| `key` | string | yes | | Lowercase key, at most 64 characters. |
| `label` | string | yes | | |
| `type` | `"number"`, `"boolean"`, `"text"`, `"select"`, `"secret"`, `"tls_identity"`, `"http_credential"` or `"signing_identity"` | yes | | Error: `must be one of number, boolean, text, select, secret, tls_identity, http_credential, signing_identity`. |
| `default` | any | no | none | Not allowed for any sensitive type (`secret`, `tls_identity`, `http_credential`, `signing_identity`). For `select`, one of the option values. |
| `min`, `max` | integer | no | none | Bounds for `number` settings. |
| `help` | string | no | none | Help text shown under the input. |
| `group` | string | no | none | Heading under which consecutive settings with the same group are shown. |
| `options` | array of `{ value, label }` | for `select` | `[]` | Required for `select`, not allowed for other types. |
| `options_from` | `{ entity, value, label }` | no | none | `text` settings only. The settings dialog lists the records of `entity` (a record type such as `printing.template`), storing the `value` field and showing the `label` field. Falls back to a text input when the person cannot read the entity. The value is not checked against the records. |

`tls_identity` requires `tls_origins`; `http_credential` requires `http_origins`. Both contain exact HTTPS origins and are rejected on other setting types. `signing_identity` requires `signing_algorithms`; a TLS identity may opt into signing with the same field. Supported algorithms are `rsa_pkcs1_sha256`, `rsa_pkcs1_sha512`, `rsa_pkcs1_sha1` (explicit legacy opt-in) and `ed25519`. Signing algorithms are rejected on other types.

See [Settings and secrets](/docs/plugins/settings-and-secrets/).

## `[[permissions]]` and `[[role_presets]]`

| Key | Type | Required | Default | Rules |
| --- | --- | --- | --- | --- |
| `permissions[].key` | string | yes | | Must start with `<plugin key>.`, be a valid permission (lowercase letters, digits, `_`, dot-separated, at most 100 characters) and not already exist in Aphel. Error: `must be <plugin key>.<action> with lowercase letters and underscores`. |
| `permissions[].label` | string | yes | | 1 to 120 characters. |
| `permissions[].description` | string | no | none | |
| `permissions[].grant_to` | array of strings | no | `[]` | Role keys that receive the permission with organization scope when the plugin is installed or its grants are reapplied. Each must be a lowercase key of at most 64 characters. A role that does not exist is skipped. |
| `role_presets[].key` | string | yes | | Unique lowercase role key, at most 64 characters, not `admin`. |
| `role_presets[].label` | string | yes | | 1 to 140 characters. Becomes the role name. |
| `role_presets[].description` | string | no | none | |
| `role_presets[].permissions` | array of strings | yes | | At least one. Each must be a declared permission, a `requires` or `optional_requires` entry, or a permission of the plugin's entities. |
| `role_presets[].scope` | `"own"`, `"team"` or `"organization"` | no | `"organization"` | Scope of the granted permissions. |
| `role_presets[].home` | string | no | none | Application path starting with a single `/`, at most 200 characters, used as the landing page of users with that role. |

At most 20 role presets. See [Permissions and roles](/docs/plugins/permissions-and-roles/).

## `[http]`

| Key | Type | Required | Default | Rules |
| --- | --- | --- | --- | --- |
| `allowed_hosts` | array of strings | no | `[]` | At most 20 lowercase host names with at least two labels, such as `api.example.com`. `*.example.com` matches subdomains of `example.com` but not `example.com` itself. Error: `must be a lowercase host name such as api.example.com or *.example.com`. |

## `[[actions]]`, `[[emits]]`, `[[notifications]]`, `[[widgets]]`

| Key | Type | Required | Default | Rules |
| --- | --- | --- | --- | --- |
| `actions[].key` | string | yes | | Unique lowercase key, at most 64 characters. At most 20 actions. Becomes the automation action `plugin.<plugin key>.<key>`. |
| `actions[].label` | string | yes | | 1 to 120 characters. |
| `actions[].description` | string | no | none | |
| `actions[].record_types` | array of strings | no | `[]` | Record types the action supports. Empty means all record types that automations can use. |
| `actions[].retry_safe` | boolean | no | `false` | Whether automations may retry the action. |
| `actions[].inputs[]` | array of tables | no | `[]` | At most 20 per action. |
| `actions[].inputs[].key` | string | yes | | Unique lowercase key, at most 64 characters. |
| `actions[].inputs[].label` | string | yes | | |
| `actions[].inputs[].type` | `"text"`, `"number"`, `"boolean"` or `"select"` | no | `"text"` | |
| `actions[].inputs[].required` | boolean | no | `false` | |
| `actions[].inputs[].options` | array of strings | for `select` | `[]` | Required for `select`. |
| `emits[].name` | string | yes | | Unique lowercase key, at most 64 characters. At most 50. Becomes the event `plugin.<plugin key>.<name>`. |
| `emits[].label` | string | yes | | 1 to 120 characters. |
| `emits[].description` | string | no | none | |
| `emits[].record_types` | array of strings | no | `[]` | Record types the event may be emitted for. Empty means any. |
| `notifications[].name` | string | yes | | Unique lowercase key, at most 48 characters. At most 20. The notification kind is `<plugin key>_<name>`. |
| `notifications[].label` | string | yes | | 1 to 120 characters. |
| `notifications[].description` | string | no | none | |
| `notifications[].channels` | array of strings | no | all three | Some of `in_app`, `email`, `push`. Cannot be empty. |
| `notifications[].email_default` | boolean | no | `false` | Whether email is on by default. |
| `notifications[].push_default` | boolean | no | `false` | Whether push is on by default. |
| `widgets[].name` | string | yes | | Unique lowercase key, at most 48 characters. At most 20. The widget key is `<plugin key>_<name>`. |
| `widgets[].title` | string | yes | | 1 to 120 characters. |
| `widgets[].category` | string | no | `"plugins"` | Lowercase key, at most 64 characters. |
| `widgets[].permission` | string | no | none | Must be one of the plugin's capabilities. Only users holding it see the widget. |
| `widgets[].size` | `"small"`, `"medium"` or `"large"` | no | `"medium"` | |
| `widgets[].uses_period` | boolean | no | `true` | Whether the dashboard period selector applies. |
| `widgets[].default_settings` | table | no | `{}` | Must be a table. |

See [Automation and widgets](/docs/plugins/automation-and-widgets/).

## `[menu]` and `[privacy]`

`[menu]` rejects unknown keys.

| Key | Type | Default | Rules |
| --- | --- | --- | --- |
| `menu.flat` | boolean | `false` | Read by the web navigation. See [Web screens](/docs/plugins/web-screens/). |
| `menu.own_entities` | `"visible"` or `"hidden"` | none | `"hidden"` hides the plugin's own entities from the navigation, with or without a web bundle. Error: `must be one of visible, hidden`. |
| `privacy.subject_data` | boolean | `false` | When `true`, the backend answers data subject requests. Requires a backend. |

## `[web]`

| Key | Type | Default | Rules |
| --- | --- | --- | --- |
| `web.entry` | string | `"web/index.js"` | ES module loaded by the Aphel web app. Must exist in the package. |
| `web.style` | string | none | Stylesheet path inside the package. |
| `web.public` | string | none | Directory of a public single-page application, inside `web/`, containing `index.html`. Served at `/p/<plugin key>/`. |

Declaring `[web]` requires an explicit acknowledgement on every install, upgrade and enable (`plugin_web_unacknowledged`). See [Web bundle](/docs/plugins/web-bundle/), [Public web apps](/docs/plugins/public-web-apps/) and [Security model](/docs/plugins/security-model/).

## `[provides]`, `[uses]` and `[[contribute]]`

These three tables reject unknown keys.

| Key | Type | Default | Rules |
| --- | --- | --- | --- |
| `provides.contracts` | array of strings | `[]` | Contract requirements such as `name@1`. A contract that a built-in module declares makes the plugin hold that module: the module is disabled while the plugin is enabled. |
| `provides.operations[].key` | string | | Dotted lowercase segments such as `module.verb`, at most 96 characters, unique. At most 64 operations. Not already provided by a built-in module (unless the plugin holds that module) or another plugin. |
| `provides.operations[].description` | string | | 1 to 240 characters. |
| `provides.operations[].effect` | `"read"`, `"write"` or `"external"` | `"read"` | |
| `provides.operations[].permission` | string | none | A valid permission the caller must hold. |
| `provides.operations[].idempotent` | boolean | `false` | |
| `uses.required` | array of strings | `[]` | Contracts that must be provided by an enabled module or plugin, otherwise `plugin_contract_unavailable`. |
| `uses.optional` | array of strings | `[]` | Contracts used when available. |
| `contribute[].point` | string | | Dotted lowercase segments, for example `dashboard.widget`. At most 128 contributions. |
| `contribute[].name` | string | | Lowercase key, unique within its point. |
| `contribute[].title` | string | none | |
| `contribute[].data` | any | null | Data passed with the contribution. |

An operation provided by a plugin needs a backend with `on-operation`. See [Automation and widgets](/docs/plugins/automation-and-widgets/) and [Lifecycle](/docs/plugins/lifecycle/).

## Limits at a glance

| Item | Maximum |
| --- | --- |
| Plugin key | 32 characters |
| Name | 120 bytes |
| Schedules | 10 |
| Actions | 20 (20 inputs each) |
| Emitted events | 50 |
| Notification kinds | 20 |
| Widgets | 20 |
| Allowed hosts | 20 |
| Operations used with `invoke` | 100 |
| Role presets | 20 |
| Provided operations | 64 |
| Contributions | 128 |

## Reserved and rejected keys

| Where | Rejected | Why |
| --- | --- | --- |
| `key` | `custom` | Prefix of administrator-defined objects. |
| `key` | Any module key compiled into this Aphel | Built-in modules own their keys. |
| `key` | `core`, `default` when `[ui]` is declared | Reserved for interface plugins. |
| Top level | `conflicts_with` | Removed. Provide the contract of the module you replace through `provides.contracts`. |
| `role_presets[].key` | `admin` | The administrator role is not assignable by plugins. |
| `permissions[].key` | Keys not prefixed by `<plugin key>.`, or keys already in the permission catalog | Plugins can only create their own permissions. |
| Managed entity field keys | `id`, `created_at`, `updated_at`, `created_by`, `owner_id`, `deleted_at`, `archived_at` | System columns. |
| Dynamic entity field keys | `name`, `title`, `owner_id`, `created_by`, `created_at`, `updated_at` | System fields and the generated title field. Conflict: `plugin_field_conflict`. |
| `routes[].path` | Anything containing `..` or not starting with `/` | |
| `hooks.declare[].key` | Keys outside the `<plugin key>.` namespace | |
| `provides.operations[].key` | A key owned by a built-in module or another plugin | |
