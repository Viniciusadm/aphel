---
title: Roles and permissions
description: The built-in roles, how permissions and scopes work, field-level access, record sharing and API token scopes.
sidebar:
  order: 4
---

Access in Aphel is a set of **permissions** granted to **roles**. Each member has one role. Roles are managed under Settings, Roles and need the `settings.manage` permission.

## Permissions

A permission is a dotted key such as `people.read` or `settings.manage`. Modules add their own permissions to the catalog when they are enabled. The Roles screen (and `GET /api/v1/permissions`) lists every permission available in the installation. The full list is in the [permissions catalog](/_aphel/docs/reference/permissions-catalog/).

There are two kinds:

- **Record permissions** act on records of a type (read, create, update, delete, export and so on). They are granted with a scope.
- **Administrative permissions** have no scope. The kernel defines these:

| Permission | Allows |
| --- | --- |
| `settings.manage` | Organization settings, roles, teams, modules, field and permission configuration, configuration packages |
| `users.manage` | Members, invitations, organization-wide security events, listing every member's API tokens |
| `audit.read` | Reading the audit log |
| `system.manage` | The System screen: health, job queue, failed jobs, migrations, recent failures |
| `records.share` | Sharing any record, not only the ones you own |

Modules add more, for example `privacy.manage` for data subject requests.

## Scopes

A record permission is granted with one of three scopes. The scope decides which records the permission reaches:

| Scope | Reaches |
| --- | --- |
| `own` | Records the user owns |
| `team` | Records owned by members of the user's teams and of their sub-teams |
| `organization` | Every record of that type |

Records a user can reach through [sharing](#record-sharing) are added on top.

## Built-in roles

Three system roles are created at installation. Their keys are `admin`, `manager` and `member`.

| Role | Key | Grants |
| --- | --- | --- |
| Administrator | `admin` | Every permission, at the organization scope. This role cannot be edited or deleted. |
| Manager | `manager` | Record permissions at the `team` scope |
| Member | `member` | Record permissions at the `own` scope, except export permissions |

For a record permission that no module declares grants for, `manager` and `member` receive it at the scope shown above. When a module declares grants for a permission, only the roles it lists receive it, and its declaration can make the grant organization-wide. Administrative permissions such as `settings.manage` are never granted to `manager` or `member` unless a module declares it. The `records.share` permission is granted to administrators only. Module grants are applied when the module is enabled.

The display names of the three roles are seeded when the organization is created and are `Administrador`, `Gerente` and `Colaborador` in the current version. The keys never change. You can rename a role.

You can edit the permissions of `manager` and `member`, but not their keys, and you cannot delete any system role. Aphel refuses to delete a role that is assigned to members (`role_in_use`).

## Custom roles

Create a role with a name, an optional key (lowercase letters, digits and underscores, up to 64 characters) and a list of permissions with scopes. Keys must be unique, and keys starting with `platform_` are reserved. Administrative permissions take no scope. Changes to a role's permissions are recorded as security events.

## Field-level permissions

Beyond which records a role can reach, you can restrict individual fields. Under Settings, Fields, open the permission matrix of a record type. For every field and role you choose one of:

| Access | Effect |
| --- | --- |
| `editable` | The default. Read and write. |
| `read_only` | The field is returned but cannot be changed (`422 field_read_only`). |
| `hidden` | The field is removed from responses, including custom fields, embedded objects and webhook bodies. It cannot be filtered, sorted, searched, exported, imported or written (`422 field_not_accessible`), and it shows as `[hidden]` in history and audit diffs. |

The `admin` role is always `editable` and not configurable. Locked fields, such as a record's name or title, owner and timestamps, the pipeline, stage and status of a deal, and required custom fields with no default value, are always `editable`. The matrix is saved as a whole: any cell you leave out becomes `editable`. Use `If-Match` with the matrix `updated_at` when saving through the API. The matrix API is `GET` and `PUT /api/v1/field-permissions/{record_type}`.

## Record sharing

Sharing gives a user or a team access to a single record on top of their role.

- **Who can share:** the owner of the record, or any member with `records.share`, who can read the record.
- **Access levels:** `read` adds the record to every read, search, report and export of the grantee. `edit` also lets the grantee update, move and archive it, provided their role has the update permission for that type.
- A share never allows deleting the record or sharing it again.
- The limit is 100 shares per record. Sharing is audited and recorded in the security log.

The API is `GET` and `POST /api/v1/records/{record_type}/{record_id}/shares`, with `PATCH` and `DELETE` on a share.

## API token scopes

A personal API token acts as its owner, narrowed by the token's scopes, and it can never do more than its owner can at the time of each request.

- The scopes a user can grant are derived from their permissions. A read permission such as `people.read` becomes `people:read`. Any other action on a resource becomes `<resource>:write`. `GET /api/v1/api-tokens/scopes` lists them.
- A token is created with a name (1 to 100 characters), scopes and an optional expiry. The secret has the form `aphel_<prefix>_<secret>`, is shown once, and is sent as `Authorization: Bearer <secret>`.
- A user can have at most 50 active tokens.
- Tokens cannot manage sessions, two-factor authentication, passwords or other tokens: those endpoints answer `403 api_token_not_allowed`.
- Members revoke their own tokens. Users with `users.manage` can list and revoke every token.

See [Security](/_aphel/docs/admin/security/) for token rate limits and [Authentication](/_aphel/docs/reference/authentication/) for how to call the API.
