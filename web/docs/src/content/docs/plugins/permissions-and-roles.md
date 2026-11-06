---
title: Permissions and roles
description: How plugins declare permissions, how capabilities limit what a plugin can do, and how grants and role presets reach roles.
sidebar:
  order: 5
---

Aphel authorizes every action with permissions held by roles. A plugin takes part in this system in three ways: it declares which permissions it needs, it can create permissions of its own, and it can suggest roles. This page explains each one and the rule that ties them together: a plugin acts with the intersection of the actor's permissions and the plugin's capabilities.

## Capabilities

The capabilities of a plugin are the permissions it is allowed to use. They are the union of:

| Source | Example |
| --- | --- |
| `requires` | `deals.read` |
| `optional_requires` | `tasks.create` |
| The keys of `[[permissions]]` | `library.audit` |
| Every permission of the plugin's own entities | `library.book.read`, `library.book.create`, `library.book.update`, `library.book.delete`, `library.book.archive`, `library.book.export` |

Anything not in this list is invisible to the plugin: a plugin that does not list `people.person.read` cannot read people, even when the user who triggered it can.

`requires` entries must be permissions that exist in this Aphel (or permissions of an administrator-defined custom object), otherwise the manifest is rejected with `is not a known permission`. The module that owns a required permission must be enabled when the plugin is installed or enabled, otherwise the install fails with `422 plugin_dependency_unavailable`. `optional_requires` entries only need to be valid permission names; they never block the install.

## Actor ∩ capabilities

Every call into the plugin builds a permission set from the actor and the capabilities.

1. Take the permissions of the actor: a user, an API token (its scopes already intersected with its creator's permissions), an automation, or, when nobody is signed in, the user who installed the plugin.
2. Keep only the permissions that are also capabilities of the plugin.
3. Keep the record scope (`own`, `team`, `organization`) the actor has for each permission that remains.

The host functions use this set for everything: record reads and writes, `can`, `permissions`, `invoke`, `emit` and so on. A record created by the plugin on behalf of a user the user could not create directly is refused, and the reverse is true as well: the plugin cannot do more than it declared.

`can(permission)` answers whether the intersection grants the permission, with any scope. `permissions()` returns the whole intersection as a list of permission keys. Use them to adapt behavior, not to authorize: the host already enforces permissions on every record operation.

Which actor is used in each entry point is described in [Security model](/docs/plugins/security-model/).

## Permissions of your entities

Declaring an entity creates its six permissions automatically. The first time the entity is created, the host also grants them to the system roles:

| Role | Permissions | Scope |
| --- | --- | --- |
| `admin` | all | organization (the administrator always holds everything) |
| `manager` | `read`, `create`, `update`, `delete`, `archive`, `export` | `permission_scope` of the entity, or `team` when not set |
| `member` | `read`, `create`, `update`, `delete`, `archive` | `permission_scope` of the entity, or `own` when not set |

Reinstalling or reapplying grants adds the same grants again for entities that declare `permission_scope`. Existing grants are never changed or removed.

## Permissions you declare

Use `[[permissions]]` for actions that are not about one entity, for example approving something, running an export route or opening the plugin settings.

```toml
[[permissions]]
key = "library.audit"
label = "Audit the library"
description = "See the audit report and run the stock check."
grant_to = ["manager"]
```

Rules:

- The key must start with `<plugin key>.`, contain only lowercase letters, digits and underscores in dot-separated segments (at most 100 characters), and must not exist in Aphel already. A key that is already registered fails the install with `409 permission_already_registered`.
- `label` has 1 to 120 characters.
- `grant_to` lists role keys. When the plugin is installed or its grants are reapplied, those roles receive the permission with organization scope. Roles that do not exist are skipped. Do not list `admin`: it already holds everything.

Declared permissions are stored in the permission catalog with the owner `plugin:<key>`. They are active only while the plugin is enabled, appear in the role editor with your label and description, and are removed when the plugin is uninstalled.

Use a declared permission anywhere the manifest takes a permission: `routes[].permission`, `widgets[].permission`, `settings_permission`, `role_presets[].permissions` and `provides.operations[].permission`.

## Role presets

A role preset is a suggestion for a role that suits the plugin, for example a `librarian` role. Presets are offered, never forced.

```toml
[[role_presets]]
key = "librarian"
label = "Librarian"
description = "Manages books and sees the audit report."
permissions = ["library.book.read", "library.book.create", "library.book.update", "library.audit"]
scope = "team"
home = "/books"
```

| Key | Meaning |
| --- | --- |
| `key` | Role key. Must be lowercase, unique in the manifest and not `admin`. |
| `label` | Role name, 1 to 140 characters. |
| `permissions` | At least one: declared permissions, `requires`/`optional_requires` entries and permissions of your entities. |
| `scope` | `own`, `team` or `organization` (default). The scope of every permission in the preset. |
| `home` | Application path where users with this role land. Starts with a single `/`, up to 200 characters. |

In the consent dialog the presets are listed **unchecked**. The administrator selects the ones to create. Through the API, send the keys in the multipart field `role_presets`, comma separated.

```bash
curl -sS -X POST "$APHEL_URL/api/v1/wasm-runtime" \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -F "package=@library-0.1.0.aphelplugin" \
  -F "role_presets=librarian"
```

- A selected preset creates the role if no role with that key exists. The new role is not a system role.
- If a role with the key already exists, it is reused and receives the preset's permissions.
- A key that is not a preset of the package fails with `422 plugin_role_preset_unknown`.
- Presets that were not selected create nothing. If an administrator creates a role with a preset's key later, a reapply (below) grants its permissions to it.
- Two enabled plugins that offer a preset with the same key are reported as a conflict that must be acknowledged. See [Lifecycle](/docs/plugins/lifecycle/).

## Reapplying grants

Roles and plugins change independently. To grant again what the manifest declares, call:

```bash
curl -sS -X POST "$APHEL_URL/api/v1/wasm-runtime/library/grants" \
  -H "Authorization: Bearer $APHEL_TOKEN"
```

This needs `system.manage`. It adds missing grants for `grant_to`, for presets and for entity `permission_scope` defaults, to the roles that exist now. It never creates roles and never removes grants. It publishes `plugin.grants_reapplied` with an audit entry and returns the plugin with its grant status.

The plugin object, the consent response and the reapply response report, per role:

```json
{
  "role": "librarian",
  "label": "Librarian",
  "preset": true,
  "role_exists": true,
  "expected": 4,
  "granted": 3,
  "missing": ["library.audit"]
}
```

`ungranted_roles` lists the roles that do not exist or are missing grants. The Plugins page shows the same information.

## Designing permissions

- Request the least. Each `requires` entry appears in the consent dialog.
- Prefer entity permissions to custom permissions when the action is about a record type.
- Gate routes and widgets with `permission` instead of checking `can` in the code.
- When your backend calls module operations with `invoke`, list the permissions those operations need in `requires`, because the intersection applies to the operation as well.
- Remember that records are filtered by scope: a user with `library.book.read` at `own` scope sees only their books, in the plugin as everywhere else.
