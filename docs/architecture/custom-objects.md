# Custom objects and relationships

The organization can create its own entities (contracts, properties, equipment, projects, renewals) and link any record types to each other. A custom object becomes a record type just like the core ones: it enters the installation's `RecordTypeRegistry` and, from then on, lists, filters, views, layouts, reports, global search, import, export, timeline, notes, attachments, tags, field permissions, API, webhooks and configuration packages work without specific code.

## Pieces

| Piece | Where it lives | What it does |
| --- | --- | --- |
| `CustomObject`, `CustomRecord`, `RelationshipDefinition` | `back/kernel/src/domain/custom_objects` | Entities, keys (`custom.<key>`), generated permissions (`custom.<key>.<action>`), cardinality and `on_delete` |
| `custom_record_definition`, `build_registry` | `back/kernel/src/application/custom_objects/definition.rs` | Builds the definition of the `custom.<key>` type and adds the relationships' relations to the installation registry |
| `RecordTypeCatalog` | `back/kernel/src/application/custom_objects/catalog.rs` | Registry of the **installation's** types (static core + objects and relationships from the database), with a single versioned cache |
| `CatalogJobHandler`, `CatalogEventConsumer` | `back/kernel/src/application/custom_objects/runtime.rs` | Worker jobs and consumers that resolve the current registry on each run |
| `CustomObjectAdminUseCases` | `back/kernel/src/application/custom_objects/admin.rs` | Objects and relationships (`settings.manage`) |
| `CustomRecordUseCases` | `back/kernel/src/application/custom_objects/records.rs` | Generic record CRUD |
| `RelationUseCases` | `back/kernel/src/application/custom_objects/relations.rs` | List, link and unlink records of any type |
| MySQL repositories | `back/kernel/src/infrastructure/custom_objects` | `custom_objects`, `relationship_definitions`, `record_relations`, `custom_records`, schema source and system field sync at boot |
| `custom_objects` section | `back/kernel/src/application/config_packages/sections/custom_objects.rs` | Configuration packages |
| Routes | `back/kernel/src/api/routes/v1/custom_objects.rs` | `/custom-objects`, `/relationships`, `/objects`, `/objects/{key}/…`, `/records/{type}/{id}/relations…` |
| Screens | `web/src/features/settings/objects`, `web/src/features/custom-records` | Administration at `/settings/objects`, navigation, list, Kanban, calendar and detail with relationships |

## Tables

- **`custom_objects`** (migration `1600`): `key` (unique, immutable), singular and plural labels, description, icon, color, `title_field_key`, `owner_enabled`, `is_active`, `position`.
- **`custom_records`** (`1601`): a single table for the records of all objects, with `object_id`, `owner_id`, `title` (denormalized), `archived_at`, `deleted_at` and dates. All other data lives in `custom_field_values`/`custom_field_multi_values`, already typed and indexed (session 04), with `record_type = custom.<key>`.
- **`relationship_definitions`** (`1602`): `key`, `inverse_key`, `from_type`, `to_type`, `cardinality`, `from_label`, `to_label`, `is_required`, `on_delete`.
- **`record_relations`** (`1603`): links `(relationship_id, from_id, to_id)`, indexed in both directions.
- **`organization_settings.custom_schema_version`**: version of the custom schema, incremented in the same transaction as every object or relationship change.

## The installation's type registry

The `RecordTypeRegistry` is not static: `AppState.record_types` is a `RecordTypeCatalog`, a single cache for the installation:

- `builtin()` returns the core registry (contact, company, deal, activity, product), used where only core types make sense (workflow, forecast, goals, duplicates, incoming webhooks, demo).
- `registry()` (on `AppState`, `state.registry().await?`) returns the current registry: core + one `EntityDefinition` per **active** object + one `RecordRelation` on each end of each relationship. The registry also carries the schema (`schema()`/`state.custom_schema()`: objects and relationships).
- The cache stores a single registry together with the `custom_schema_version` (`organization_settings`). Each resolution reads the version (a primary key lookup) and rebuilds only when it has changed, so `serve`, `worker` and multiple instances see the change on the next request, without relying on messages between processes. The administration handlers still call `invalidate` (`state.record_types.invalidate()`) in the process that made the change.

In the HTTP routes, the helpers that build ports for generic resources are asynchronous and read `state.registry()` (`field_use_cases(&state, &scope)`, `aggregate_ports`, `view_use_cases`, `layout_use_cases`, `tag_use_cases`, `note_use_cases`, `file_use_cases`, `timeline_use_cases`, `report_use_cases`, `segment_use_cases`, `favorite_use_cases`, `field_permission_use_cases`, `config_package_use_cases`, `Ctx::from_state` for import/export/bulk actions, search, and also people, organizations, deals and products, so that the core lists can filter by relationships). In the worker, `CatalogJobHandler`/`CatalogEventConsumer` build the handler with the current registry on each job or event (export keeps the `AppState` from `exports::worker_state` and builds `Ctx::from_state` on each job; import, bulk actions, scheduled report delivery, search index). **New code that receives a `record_type` coming from the user must use `state.registry()`**, never `RecordTypeRegistry::builtin()`.

### Definition of the `custom.<key>` type

| Property | Value |
| --- | --- |
| Table | `custom_records`, with **partition** `object_id = <object id>` (`EntityDefinition::partition`) |
| Permissions | `custom.<key>.read` (read) and `custom.<key>.update` (edit); also `create`, `delete`, `export`, `archive` |
| Owner | `owner_id` when `owner_enabled`; without an owner, only the organization scope can see it (like the product catalog) |
| Deletion/archiving | `deleted_at` and `archived_at` |
| System fields | `title` (read-only), `owner_id` (if there is an owner), `created_by`, `created_at`, `updated_at` |
| Title | `title_field` = `title`; `title_column()` is used for link labels and search |

The partition is applied by `push_record_scope_as` (and therefore by `push_live_record`, `push_where` and all SQL that builds the read scope) and, in the few places that read the table without a scope (search source, export), by `push_partition`. Anyone who copies a definition to another table (such as the search index header) must clear the partition.

## Objects

- **Key**: lowercase letters and `_`, up to 40, immutable; the type is `custom.<key>` and the API path is `/objects/<key>/records`.
- **Creation** (`settings.manage`, event `custom_object.created`): in the same transaction it writes the object, seeds the system fields in `field_definitions` (as `sync_system_fields` does at boot for the core; `sync_custom_object_fields` repeats this at the boot of `serve`/`worker`/`migrate`), creates the required custom field `name` ("Nome", searchable), grants the permissions to the system roles (manager: everything with team scope; collaborator: everything except export, own scope; administrator already has everything) and increments the schema version.
- **Title field**: `title_field_key` points to an active custom field of type text, email, phone or URL. Each record write copies the normalized value of that field to `custom_records.title`; changing the field recalculates the existing titles and reindexes the type's search.
- **Deactivating** (`is_active: false`) removes the type from the registry: it disappears from navigation, the API and all generic resources, while keeping the data. **Deleting** is only allowed with no live records and deletes the type's fields, views, layouts, segments, reports, field permissions, role permissions and relationships.
- **Limits** fixed in code (`kernel/application/custom_objects/ports.rs`): `CUSTOM_OBJECTS_MAX = 50` objects created by users (objects with a `module_key`, created by modules and runtime plugins through `CustomObjectAdminUseCases::create_managed`, are not counted) and `CUSTOM_OBJECT_FIELDS_MAX = 200` custom fields per object.

## Stages (`workflow.staged`)

`PATCH /api/v1/custom-objects/{key}` with `"staged": true` gives an unmanaged object the `workflow.staged` trait (`stage_field = "stage"`, stored in the object's `entity_definition`) and creates the `stage` text field when it does not exist. The web then shows a Stages view next to List: columns are the stages of the chosen workflow pipeline and dragging a card writes the stage id to the `stage` field. `staged: false` removes the trait and keeps the field and its values.

The `objects` module contributes a `StagedRecordSource` for every staged object, so the workflow module counts records per stage, blocks deleting a stage or pipeline in use and moves the records to the target stage when one is chosen. It does not own the pipeline or rotting columns of deals: rotting, entry/exit required fields and stage events are not applied to custom objects yet.

## Records

`/api/v1/objects/{key}/records` follows the style guide: `{ data }`/`{ data, meta }`, `filter` (AST or `filter[field][op]=value`), `sort`, `search`/`q`, `archived`, `If-Match` on PATCH, events `custom.<key>.created|updated|deleted|archived|unarchived` with audit in the same transaction. Values go in `custom_fields` (validated by the `CustomFieldWriter`, with layout conditional rules and field permissions); the default `owner_id` is the creator. `GET /objects/{key}/schema` documents at run time the fields (as the caller sees them), the relations and the events; the static OpenAPI only documents the generic pattern.

Deletion is soft and stays **outside the trash** (like products): the record disappears from lists and its links are removed in the same transaction.

## Relationships

A relationship links any two types among `company`, `contact`, `deal`, `product` and active custom objects (including a type with itself).

| Property | Meaning |
| --- | --- |
| `key` / `inverse_key` | Name of the relation on the `from` type and on the `to` type (`company.contracts`, `custom.contract.company`). They cannot collide with system fields or with other relations of the type |
| `cardinality` | `one_to_one`, `one_to_many` (one `from` to many `to`; each `to` has a single `from`) or `many_to_many` |
| `from_label` / `to_label` | Label shown on the `from` record (e.g. "Contracts" on the company) and on the `to` record (e.g. "Company" on the contract) |
| `required` | Every `to` record of a custom object needs a link: required on creation (`relations` in the POST) and the last link cannot be removed (409 `relation_required`) |
| `on_delete` | When deleting a `from` record: `restrict` blocks while there are live linked `to` records (409 `record_has_relations`), `unlink` allows the deletion. Deleting a `to` record always unlinks |

The lock is checked inside the deletion transaction by the `RecordDeleting` hook handler (`RecordLinkRepository::ensure_deletable`, registered in `back/modules/record_hooks.rs`), which runs on every deletion through the record pipeline (organizations, people, deals, tasks, products, quotes and custom records). Deleted core records go to the trash with their links intact (restoring brings the links back); the permanent purge deletes the links. Cardinality is guaranteed with `SELECT … FOR UPDATE` on the links of both ends before inserting.

Links: `GET /records/{type}/{id}/relations` (the record's relations with the visible total), `GET …/relations/{relation}` (linked records the caller can read), `POST …/relations/{relation}` `{ record_id }` (requires editing the record and reading the other one) and `DELETE …/relations/{relation}/{target_id}`. Linking and unlinking publish `record.linked`/`record.unlinked` on the source record, with audit.

**Fields of type `record`** still exist for the core's simple links, but they cannot point to custom objects: use relationships for that.

## Queries, reports and search

- **Relations in the type registry**: each end becomes a `RecordRelation` with `RelationPath::Relationship { relationship_id, inverse, single }`, compiled as `EXISTS` over `record_relations` with the read scope and the related record's deletion. That is why, with no new code, the following work: `relation.field` filters (`company.industry` on contracts, `contracts.stage` on organizations), the filter node `{ "related": { "relationship": "company", "filter": { … } } }` (a full filter over the related type, including groups), saved views, segments and the type's `in_saved_filter`.
- **Lists** load the `FieldSet`s of the related types cited in the filter (`load_related_sets`), including in the core lists; before session 16 the `relation.field` filters only worked in aggregations.
- **Aggregations and reports** group by a field of a **single-record** related type (`single`): contract → company (`company.industry`) does a `LEFT JOIN` on `record_relations` and on the company table (`SingleJoin::Relationship`). Metrics (`sum`, `avg`…) over the object's own numeric and currency fields.
- **Global search**: `title` and the searchable custom fields go into the index; the `search.index` consumer accepts the `custom.<key>.*` events and `custom_object.updated` reindexes the type. `aphel search reindex --record-type custom.<key>` reindexes only the object.

## Permissions

`custom.<key>.read|create|update|delete|export|archive` are record permissions (own/team/organization scope) and appear in the role matrix (`GET /permissions` = static catalog + extension with the objects). The `PermissionSet` carries this extension (`catalog_extension`), used by the administrator's `/auth/me`, by the system roles response and by the API token restriction (scopes `custom.<key>:read`/`custom.<key>:write`). Field permissions, layouts and conditional rules work as on any type.

## Integrations

- **Import**: `record_type = custom.<key>`; columns map to custom fields, `owner_id` and single-record relations (`relations.<key>`, looking up the related record by name/title). Required relations must be mapped.
- **Export**: CSV/XLSX with the visible fields, permission `custom.<key>.export`.
- **Outgoing webhooks**: events `custom.<key>.*` with `data` = `CustomRecordResponse` (concrete list in `GET /events/catalog`); see [`docs/api/events.md`](../api/events.md).
- **Tags, notes, attachments and timeline**: generic by type; edits show up as `record_change` in the timeline.
- **Configuration packages**: `custom_objects` section (objects and relationships by key), installed before `fields`; see [`docs/config-packages/README.md`](../config-packages/README.md).

## Out of scope (for now)

Trash, privacy (data subject search, anonymization and retention), bulk actions and duplicates reach custom records through the type registry (see [`trash.md`](trash.md) and [`duplicates.md`](duplicates.md)).
