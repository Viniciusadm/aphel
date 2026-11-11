---
title: Entities and fields
description: Declare record types owned by a plugin, choose their storage, add fields to built-in types and understand how upgrades treat fields.
sidebar:
  order: 4
---

A plugin stores business data as records. It can declare its own record types (entities) and it can add fields to record types that already exist. Both use the same field declarations. Keys and limits are in [Manifest](/_aphel/docs/plugins/manifest/).

## Entities

An entity declared with `[[entities]]` becomes a custom object owned by the plugin. It is not counted against the quota of objects that administrators create, and administrators cannot edit or delete it (`409 custom_object_managed_by_module`).

| Name | Value |
| --- | --- |
| Record type | `<plugin key>.<entity key>`, for example `library.book` |
| Object key | `<plugin key>_<entity key>`, for example `library_book` |
| Permissions | `<record type>.read`, `.create`, `.update`, `.delete`, `.archive`, `.export` |
| Events | `<record type>.created`, `.updated`, `.deleted`, `.archived`, `.unarchived` |
| Record API | `/api/v1/entities/<record type>/records` |

### Key rules for entities

The manifest validator accepts keys of up to 32 characters made of lowercase letters, digits and underscores. Creating the underlying object is stricter. For a plugin that declares at least one entity, the plugin key and the entity key must make an object key that:

- is at most 40 characters long,
- contains only lowercase letters and underscores (no digits),
- has no `__` and does not end with `_`.

A key that passes the manifest validator but breaks these rules fails when the object is created, with the error `invalid custom object`, and nothing is installed. Use letters and single underscores in keys of plugins that have entities. The same rules decide whether `<plugin key>.<entity key>` is recognized as a plugin record type in permissions.

Another custom object or plugin that already owns the object key makes the install fail with `409 plugin_object_conflict`.

### Choosing the storage

| `storage` | How fields are stored | Where values appear in records | Indexes | Use it when |
| --- | --- | --- | --- | --- |
| `"managed"` | Aphel generates a table with one real column per declared field and your `[[entities.indexes]]`. Never plugin SQL. | Flat: `{ "title": "Dune", "pages": 688 }` | Yes, `indexes` and `unique` | You want columns, indexes and predictable queries. Recommended. |
| `"dynamic"` (default) | Values live in typed custom field tables shared with administrator-defined objects. | Declared fields are under `custom_fields`. | No (`entity indexes require managed storage`) | Few fields and flexible schema. |

`"table"` is not available to plugins. Changing the storage of an installed entity is refused.

Managed entities get these system fields in addition to the declared ones.

| Field | Type | Notes |
| --- | --- | --- |
| `id` | uuid | |
| `created_at`, `updated_at` | datetime | `updated_at` is the version used by `update-record-if`. |
| `created_by` | user | Read-only. |
| `owner_id` | user | Only when `owner_enabled` is `true` (default). Set to the creating user when omitted. |
| `archived_at`, `deleted_at` | datetime | Archive and soft delete. |

The title of a managed record is the field called `name`, or else the first field of type `text` or `email`. These keys are reserved and cannot be declared: `id`, `created_at`, `updated_at`, `created_by`, `owner_id`, `deleted_at`, `archived_at`.

Dynamic entities get the system fields `title`, `owner_id` (when enabled), `created_by`, `created_at`, `updated_at` and a required text custom field `name` that acts as the record title. All of these keys are reserved.

### Record shape

Records returned by `get-record`, `list-records` and the record API have this shape for a managed entity.

```json
{
  "id": "0192f0c4-7d0e-7a8a-9f3e-3c2b1a0d9e8f",
  "record_type": "library.book",
  "entity": "library.book",
  "title": "Dune",
  "isbn": "9780441172719",
  "pages": 688,
  "status": "to_read",
  "owner_id": "0192f0c3-0000-7000-8000-000000000001",
  "created_at": "2026-11-06T12:00:00Z",
  "updated_at": "2026-11-06T12:00:00Z",
  "archived_at": null,
  "custom_fields": {}
}
```

`custom_fields` holds the fields that administrators or other plugins added to your entity. Fields hidden from the caller by field permissions are removed from the record before it reaches the plugin.

To write a record, send only the values you want to set. To avoid overwriting a concurrent change, include `"_expected_updated_at"` with the record's `updated_at` in an update, or use `update-record-if` ([Host API](/_aphel/docs/plugins/host-api/)). A stale version fails with `409 version_conflict`.

## Field types

| `type` | JSON value | Options that apply |
| --- | --- | --- |
| `text` | string | `max_length` (1 to 1024), `pattern`, `default`, `unique` |
| `long_text` | string | `max_length` (1 to 65535) |
| `number` | number | `min`, `max`, `decimals` (0 to 6), `unique` |
| `currency` | number | `min`, `max`, `decimals`, `currency` (ISO 4217 code, three uppercase letters) |
| `date` | string `YYYY-MM-DD` | |
| `datetime` | string, RFC 3339 | |
| `select` | string, the option `value` | `options` (required) |
| `multi_select` | array of option values | `options` (required) |
| `checkbox` | boolean | |
| `url` | string, normalized | `unique` |
| `email` | string, normalized | `unique` |
| `phone` | string, normalized | `unique` |
| `user` | user id | |
| `record` | record id | `target` (required): the record type referenced |
| `references` | array of ids | Stored as a JSON list. Filters with `in` match records whose list holds any of the ids. A single-field index on it is a multi-valued index. Not available to fields administrators create. |

A type registered by a module through the field type registry can also be used by its key. Options that a type does not support are rejected: for example `min` on a `text` field gives `is not supported by text fields`.

### Categories and tags

An entity can keep its own selection of terms from `tagging` vocabularies, such as categories and tags, in a `references` field. Declare it with a contribution; `tagging` is then an optional dependency:

```toml
[uses]
optional = ["tagging@1"]

[[entities.fields]]
key = "draft_terms"
label = "Categorias e tags (rascunho)"
type = "references"

[[contribute]]
point = "tagging.classifiable"
name = "entry"
data = { entity = "content.entry", field = "draft_terms", published = "terms", vocabularies = ["categories", "tags"] }
```

| Key | Notes |
| --- | --- |
| `entity` | The entity that keeps the selection. |
| `field` | The `references` field people edit. On every save, ids of terms that no longer exist are dropped, and a term of a vocabulary not listed in `vocabularies` fails with `422 tag_not_allowed`. |
| `published` | Optional field that receives the selection when the record is published (for example a `publication` projection). Public site pages and filters read this one. |
| `vocabularies` | Keys of the vocabularies the entity accepts. |

A term selected in `field` or `published` of any record cannot be deleted (`409 tag_in_use`). Without `tagging`, the field keeps its values and nothing is validated.

Common options for every type: `label`, `required`, `searchable`, `personal`, `sensitive`. Mark fields that hold personal data as `personal`; `sensitive` implies `personal` and redacts the value in audit entries. See [Manifest](/_aphel/docs/plugins/manifest/) for the full table.

### Select options

Options are identified by `value`. `label` and `color` can change between versions. An option that a new version no longer declares is deactivated: existing records keep the value, but it can no longer be chosen.

### Record fields and references

A `record` field points to another record type. Writing a record field requires read access to the referenced type, so the target's read permission must be among the plugin's capabilities: list it in `requires` or `optional_requires`, or target another entity of the same plugin. Otherwise the install fails with `plugin_reference_permission_required`.

`references` on the entity add transactional behavior when the target is deleted.

```toml
[[entities.references]]
field = "author"
on_delete = "restrict"

[entities.references.target]
kind = "entity"
entity = "library.author"
```

`restrict` blocks the deletion, `clear` sets the field to empty (not allowed for required fields) and `cascade` deletes the referencing records.

A reference to an entity can also be named as a relation, so filters can look at the referenced record (for example, the site exposure of an entry that is public only while its collection is active):

```toml
relations = [{ key = "parent", field = "collection" }]
```

`field` must be listed in `references` with an `entity` target, and `key` must differ from every field key. Filters then use `{ related = { relationship = "parent", filter = { field = "state", operator = "eq", value = "active" } } }`.

### Unique fields

`unique = true` is allowed on `text`, `number`, `url`, `email` and `phone`. Uniqueness is enforced by the host when records are written, with a reservation released when the record is deleted and taken again if it is restored. Violations fail the write with the same validation error the interface shows.

Making an existing field unique through an upgrade fails when existing records already share values: the install is refused with `409 plugin_field_unique_conflict` and `details.conflicts` lists up to 20 sample record ids per field. Fix the data and install again. The dry run reports the same error.

## Extending built-in record types

Use top-level `[[fields]]` to add fields to a record type that you do not own, for example a person or a deal. The fields use the same options as entity fields, plus `record_type`.

```toml
[[fields]]
record_type = "people.person"
key = "loyalty_tier"
label = "Loyalty tier"
type = "select"
options = [
  { value = "bronze", label = "Bronze" },
  { value = "gold", label = "Gold" },
]
```

- The field is created as a custom field owned by the plugin. Users see it in forms, lists, filters and the record API under `custom_fields`.
- In hooks it appears in `custom_fields` and in `changes` as `custom_fields.loyalty_tier`.
- Extending a custom object that belongs to another plugin is refused (`cannot extend custom objects of other plugins`). You can extend your own entities with `[[fields]]` as well.
- If a field with the same key already exists on that record type and is not owned by your plugin, the install fails with `409 plugin_field_conflict`.
- Fields added by a plugin cannot be edited or deleted by administrators while the plugin owns them (`409 field_managed_by_module`).
- A field whose record type does not exist fails with `invalid plugin field`.

## Upgrades, deactivation and type changes

When you install a new version of an installed plugin, Aphel reconciles the declarations.

| Change in the new version | Result |
| --- | --- |
| New entity, field or option | Created. |
| Label, `required`, `searchable`, `unique`, constraints or option labels changed | Updated. |
| Field no longer declared | Deactivated. Its data is kept and it comes back if a later version declares it again. |
| Select option no longer declared | Deactivated. Existing values are kept. |
| Entity no longer declared | The custom object is deactivated with its records. |
| Field `type` changed | Refused with `409 plugin_field_type_changed`. Declare a field with a new key. |
| Field becomes `unique` while duplicates exist | Refused with `409 plugin_field_unique_conflict`. |
| Entity `storage` changed | Refused (`changing entity storage needs a data migration`). |

The type of a field never changes, even between compatible types such as `text` and `email`: the new type string must equal the stored one. Disabling a plugin deactivates all its entities and fields until it is enabled again. Uninstalling without purge deactivates them too; `purge=true` deletes records, tables and field definitions. See [Lifecycle](/_aphel/docs/plugins/lifecycle/).

Existing records of deactivated fields are not rewritten, so be careful when reusing a field key with a different meaning.
