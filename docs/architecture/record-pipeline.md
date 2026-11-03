# Record pipeline, generic hooks and per-type handlers

Every record write (contact, company, deal, activity, product, quote, custom object and lead transitions) goes through the kernel's **record pipeline**. In it, any module can intercept, change or veto the creation, update, deletion, querying and serialization of any record type, without editing the owning module. Code in `back/kernel/src/records/`.

## Write stages

The use case still owns authorization and persistence. It receives the `RecordPipeline` port (`fields().pipeline()`, contract in `kernel/application/fields/pipeline.rs`, implementation `MySqlRecordPipeline` in `kernel/records/pipeline.rs`) and calls the stages in this order:

| Stage | Method | Hook | Can |
| --- | --- | --- | --- |
| 1. Authorization | `scope.require(..)` in the use case | — | — |
| 2. Before writing | `pipeline.saving(tx, scope, RecordDraft)` | `RecordSaving` (filter) | Change `values` (system values) and `custom_fields`, fill in defaults, normalize |
| 3. Write | `CustomFieldWriter` + the module's repository | — | — |
| 4. Veto | `pipeline.validate(tx, scope, &RecordWrite)` | `RecordValidating` (action) | Reject with 422 or 409 (`Err` aborts the transaction) |
| 5. After writing | `pipeline.saved(tx, scope, &RecordWrite)` | `RecordSaved` (action) | Write its own data in the same transaction |
| 6. Event + audit | the module's `publish_mutation`/`finish` | — | — |
| 7. Commit | `transactions().commit(tx)` | — | — |
| 8. After commit | `pipeline.committed(scope, &RecordWrite)` | `RecordCommitted` (action) | Quick side effects and cache; runs in its own transaction and an error is only logged |

Deletion: `pipeline.deleting(tx, scope, &record)` (`RecordDeleting`, can block) before deleting and `pipeline.deleted(tx, scope, &record)` (`RecordDeleted`) afterwards, in the same transaction. Regular deletion is logical (`deleted_at`, goes to the trash) and deletes neither custom values nor links, so that restoring works. **Permanent** deletion (emptying the trash, right to be forgotten, retention) goes through `pipeline.purged(tx, scope, &record)`, called by `CustomFieldWriter::purge_record`: it runs the `RecordPurged` hook (action, in the transaction, so modules can delete their own data about the record) and then the kernel deletes the record's `custom_field_values`, `custom_field_multi_values`, `record_relations`, attachment links (`attachments`) and shares (`record_shares`), clears `record`-type custom fields of other records that pointed to it and deletes the `files` rows left without references. The return value (`PurgedRecord { orphan_storage_keys }`) carries the keys of those files so the caller can remove them from `FileStorage` after the commit (the trash does this). The caller only deletes the row of the record itself. The data a module keeps about the record is removed by the module itself in a `RecordPurged` handler (`notes`: notes; `tags`: `taggings`; `activities`: `activity_links` and the direct link of activities and recurrences; `duplicates`: candidates and redirects in which the record is the primary); the trash only calls the pipeline. Restoring from the trash goes through the pipeline as an update (`deleted_at → null`): `validate` → `pipeline.restored(tx, scope, &record)` (hook `RecordRestored`, action in the transaction, so modules can reopen their own data; `leads` reopens the stage history) → `saved` → event → commit → `committed` (see [`trash.md`](trash.md)), and merging duplicates updates the primary and deletes the loser through the pipeline (see [`duplicates.md`](duplicates.md)).

Custom field uniqueness (`is_unique`): the `CustomFieldWriter` checks it inside the write transaction (`CustomFieldValueRepository::is_taken(tx, record_type, ..)`, which locks the field definition with `FOR UPDATE` until the commit) and ignores logically deleted records (the type's `deleted_column` column). That way merging duplicates, which deletes the loser before copying the values to the primary in the same transaction, can carry over a unique value from the duplicate.

Notes:

- The audit diff is computed after `RecordSaving`, so it reflects the values changed by the filters.
- `validate` sits where the old `RecordWriteGuard` used to be: on updates, before the system write when the guard needs to read the previous state from the database (approval, checklist). `saved` comes after all writes.
- Contacts, organizations, deals and lead transitions reapply the system values changed by `RecordSaving` (`ContactInput::from_values`, `CompanyInput::from_values`, `DealInput::with_record_values`; the deal recomputes stage, currency and revenue with the filtered input). Products reapply everything (`ProductInput::from_values`); activities reapply `title`, `priority`, `due_at`, `description`, `duration_minutes`, `reminder_minutes_before` and `owner_id`; quotes reapply `title`, `owner_id`, `valid_until`, `terms` and `notes` (the ones that do not affect price); custom objects reapply `owner_id` (when an owner is enabled). Reapplied values go through the same validation as the API input (422 with the field). A change to any other system value of these types (activity status and links, currency, quote price list and discount, derived object title) is rejected with the internal error `record_saving_unsupported` (`saving_unsupported(record, key)`).

## Generic hooks

Types in `kernel/records/hooks.rs`. All arguments own their data (`'static`) and carry `record_type` so the handler can filter the type it cares about.

| Hook | Argument | Where it is dispatched |
| --- | --- | --- |
| `RecordSaving` (filter) | `SavingRecord { record_type, draft: RecordDraft, registry }` | Use cases, before the `CustomFieldWriter` |
| `RecordValidating`, `RecordSaved`, `RecordCommitted` (actions) | `ChangedRecord { record_type, record, mode, changes, values, registry }` (`write()` returns the `RecordWrite`) | Use cases |
| `RecordDeleting`, `RecordDeleted` (actions) | `RemovedRecord { record_type, record, registry }` | Deletion use cases |
| `RecordPurged` (action) | `RemovedRecord { record_type, record, registry }` | Permanent deletion (`CustomFieldWriter::purge_record`), before the kernel deletes values, links, attachments, shares and references from `record` fields |
| `RecordRestored` (action) | `RemovedRecord { record_type, record, registry }` | Restore from the trash (`pipeline.restored`), after `RecordValidating` and before `RecordSaved` |
| `RecordQueryFilter` (filter) | `QueryConstraints { record_type, conditions, sort }` | `MySqlRecordQueryExecutor::execute`, dashboard aggregations and deal board totals, after the scope |
| `RecordSerializing` (filter) | `SerializedRecords { record_type, items }` | `redacted`/`redacted_items` (`kernel/api/routes/v1/field_permissions.rs`), before the `record_redactor` (which also redacts the embedded objects of each `belongs_to` relation declared on the type, by the relation key, with the target type's field policy); records embedded in another object (e.g. `company` in contact–company links) go through `serialize_nested_records(pool, scope, record_type, key, items)` (`kernel/records/pipeline.rs`) |
| `RecordSchemaFilter` (filter) | `RecordSchema { record_type, schema: FieldSchema }` | `GET /api/v1/fields/{record_type}/schema` |

`RecordQueryFilter` starts with an empty list of conditions and can only **add** conditions (combined with `AND` with the plan's already-scoped filter) and sort orders (after the existing ones). By construction it does not loosen the read scope.

The query, serialization and schema hooks run in a discarded (read-only) transaction and only when a handler is registered (`HookBus::has`).

`HookCtx` carries `tx`, `pool`, `scope`, `services` and `module`. Registration in the module's `register`:

```rust
app.on(RecordValidating, 25, |ctx, args| {
    Box::pin(async move {
        if args.record_type != RecordType::DEAL {
            return Ok(());
        }
        // ctx.tx, ctx.pool, ctx.scope, args.write()
        Ok(())
    })
});

app.filter(RecordQueryFilter, 10, |_ctx, mut constraints| {
    Box::pin(async move {
        // constraints.conditions.push(PlanNode::Condition(..));
        Ok(constraints)
    })
});
```

### Core handlers

Registered by the `kernel` module in `back/kernel/src/kernel_module.rs` (`register_record_hooks`) and by the owning modules:

| Hook | Priority | Handler |
| --- | --- | --- |
| `RecordValidating` | 10 | Approval (`ApprovalWriteGuard`): 409 `approval_pending`/`approval_required` |
| `RecordValidating` | 20 | Stage checklist (`StageChecklistGuard`) |
| `RecordValidating` | 30 | Validation rules (`ValidationRuleGuard`): 422 `validation_rule_failed` |
| `RecordValidating` | 40 | Lifecycle (`LifecycleStageGuard`): validates and writes the history |
| `RecordDeleting` | 10 | `restrict` relationships (`RecordLinkRepository::ensure_deletable`): 409 `record_has_relations` |

Approval still runs before the validation rule (409 before 422).

## Per-type handlers

`EntityDefinition.handler` holds a `RecordTypeHandler` (`kernel/records/handler.rs`) with `create`, `update`, `delete`, `load`, `archive` and `restore` over `serde_json::Value`. The body has the format of the type's API request (system fields at the top level, `custom_fields` and, for custom objects, `relations`); the response is the record already serialized and filtered by field permissions. The context is `RecordCtx { scope, state }`.

- Each owning module has its handler in `http.rs` (`ContactHandler`, `CompanyHandler`, `DealHandler`, `ActivityHandler`, `ProductHandler`, `QuoteHandler`…), calling `service::*` with `Ctx::from_state`; the custom objects one lives in `back/kernel/src/api/records.rs`. Generic helpers: `optional_record` (missing/no access becomes `None`) and `values_request::<Dto>(json)` in `kernel/records/handler.rs`. The people handler accepts `name` (split into first and last name) and the deals handler accepts `probability` as text and normalizes `currency`.
- Custom objects: one generic handler per type (`CustomRecordHandler`), created by the `app.custom_record_handler(..)` factory.
- Usage: `registry.handler(record_type)?` (`state.registry().await?`). A type without a handler responds with 422 `record_operation_not_supported`.

Who uses it: automations (`HandlerAutomationGateway`), import (`imports::writer::{create_record, update_record}`) and bulk actions (`bulk_actions::targets::apply`). A new type with a handler gets these paths without editing their code.

## Declared capabilities

What applies only to some types becomes a capability in `EntityDefinition.capabilities` (`EntityTraits`, `kernel/application/fields/capabilities.rs`), instead of a `match` on the type:

| Capability | Used by |
| --- | --- |
| `importable`, `exportable` | Import and export (permissions `<resource>.create/update/export` via `action_permission`) |
| `bulk_fields`, `supports_stages`, `supports_sequences` | Bulk actions (`move_stage`, `enroll_in_sequence`) |
| `personal_data`, `contact_columns`, `free_text_fields` | Privacy (data subjects, anonymization, retention) |
| `notifies`, `web_path`, `plural_labels` | Notifications (record summary, link and label) |
| `label_aliases` | Import by label (contact email, company domain, product code) |
| `related_records` | Email template variables |
| `locked_fields` | Field permissions (fields that cannot be hidden) |
| `restore_order` | Installation backup (restore order) |
| `automatable` | Automations (`is_automatable`: generic triggers, actions and type validation; custom objects always) |
| `validation_rules` | Validation rules (`is_validated_record_type`; custom objects always) |
| `attached_to_parent` | Records attached to another one through `record_type`/`record_id` (activities): automations treat the parent record as the target (`is_attached_to_parent`, `parent_record`, `parent_records`) |
| `relatable` | Core types that can take part in relationships with custom objects (`is_relatable_core_type`) |

Besides the capabilities, the definition carries two structural declarations:

- `hierarchy(parent_column)` (`EntityDefinition::hierarchy` or `extend(..).hierarchy(..)`): the type is a tree over its own table. It enables the `in_group` operator on the type's `id` and on `record` fields that point to it (`hierarchical_target`), compiled into `WITH RECURSIVE` over the declared column (`Predicate::InGroup(RecordGroup)`). `organizations` declares `parent_company_id`.
- `item_table(ItemTable)` (via `extend`): a table of child items that formulas aggregate (`SUM(items.total)`), with the exposed columns. `deal_items` declares `items` on `deal`.

## Registering and extending types

In the module's `register`:

```rust
app.entity(definition.handler(Arc::new(MyHandler)));

app.extend("contact", |contact| {
    contact
        .system_field(SystemField::new("temperature", "Temperatura", FieldType::Number, "temperature")?)
        .relation(relation)
        .member_access("contact_members", "contact_id", "user_id")
        .managed_field(FieldSpec::new("lead_temperature", "Temperatura", FieldType::Number)?);
});
```

- `record_type(def)` registers a type; `extend(key, |t| ..)` adds system fields (computed ones too, with `computed`), relations, `member_access`, `hierarchy`, item tables (`item_table`), managed fields and a handler to a type from another module.
- `managed_field(FieldSpec)` creates a custom field (EAV) defined in code: `sync_system_fields` (boot, `aphel migrate`, installer) writes the definition with `module_key` and, on later syncs, updates label, type and configuration. A field that no compiled module declares anymore loses its `module_key` on sync (the values stay and the administrator takes over managing it). The field appears in forms, filters, columns, import and the API like any custom field, but the UI cannot edit it: changing it, deactivating it or touching its options responds with 409 `field_managed_by_module`, and `FieldResponse.module_key` tells the owner.
- The `App` builds the final registry (`app.record_types()`), and `RecordTypeRegistry::builtin()` returns a copy of it; the `RecordTypeCatalog` adds the custom objects.
- Each module registers its own type (`organizations`, `people`, `deals`, `activities`, `products`, `quotes`); `deals` declares the deal team (`deal_members`) via `extend("deal", ..)`; `leads` and `scoring` extend `contact` and `company` (`lifecycle_stage` and `score`). Each former extension registers its own types.

## Contributed fields

A system field declared via `extend(..).system_field(..)` or `.computed(..)` is marked with `SystemField.contributed_by = Some(module)`. The column belongs to the contributing module (its migration does the `ALTER TABLE`), and the module that owns the type does not know about the field. The kernel handles these fields generically (`kernel/records/contributed.rs`):

| Function | Use |
| --- | --- |
| `load_contributed(pool, definition, ids)` | Reads the values (columns and computed expressions) of several records; the owning module's store fills `View.contributed` |
| `contributed_input(definition, values)` | Extracts, from any map (API body, values filtered by `RecordSaving`, incoming webhook), only the writable contributed fields |
| `merge_contributed(definition, current, input, validator)` | Validates the type (text, number, boolean; empty text becomes `null`) and overlays it on the current value |
| `with_contributed(snapshot, values)` | Merges the values into the record snapshot: they enter the audit diff, the field permissions and the hooks' `RecordWrite` |
| `write_contributed(tx, definition, id, values)` | `UPDATE` of the columns in the write transaction |

In the contact and company DTOs the values are flattened into the JSON (`#[serde(flatten)] contributed`, `additionalProperties` in the OpenAPI), on read and on write. Content validation belongs to the module that owns the field, via a hook (`RecordValidating`). Computed fields (`computed`) are read-only.
- `aphel modules doctor` lists, per module, the hooks and the registered or extended types.

## Entity storage and extensions

`EntityDefinition` is shared by native, managed and dynamic storage. `GenericRecordHandler` orchestrates transactional mutations, scope, field permissions, hooks, audit and outbox events; `EntityWriteBehavior` supplies transactional domain writes when an entity needs them. Contacts use that behavior for normalization, duplicates and company links. Their read, delete and archive paths use the generic handler. Native public event prefixes can remain stable through `EntityDefinition::events`.

Persistent extensions declare `RecordTypeExtension::side_table`. The extender owns the table, which uses a 1:1 record key and nullable contributed columns. The query compiler adds automatic joins; snapshots and standalone consumers use correlated expressions. Contributed writes upsert into the extender's table, and purge removes extension rows. `leads` and `lead_pro` demonstrate this without foreign `ALTER TABLE` statements. Lookup entities may declare the `records.global` trait: their read permission is still required, while the records are shared across its owner/team scopes.

Soft references are part of the definition and are checked during validation and restoration. Deletion and purge enforce `restrict`, `clear` or `cascade` in the caller's transaction. Cascaded mutations use the dependent entity's registered handler; after-commit hooks run only after the transaction commits.
