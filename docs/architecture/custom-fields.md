# Custom fields and query engine

Every Aphel record type (company, contact, deal, activity and, later, custom modules) is described by a **field registry**. The interface builds forms, columns and the filter builder from it, and listing, sorting, search and validation all go through the same engine. This document explains how the data is stored, how a filter becomes SQL and what a module needs to do to join this flow.

## Overview

| Piece | Where it lives | What it does |
| --- | --- | --- |
| `FieldType`, `FieldDefinition`, `FieldValue`, `validate_value` | `back/kernel/src/domain/fields` | Field types, configuration and value validation (pure, no database) |
| `RecordTypeRegistry` / `EntityDefinition` / `SystemField` | `back/kernel/src/application/fields/registry.rs` (native types in `builtin.rs`) | Declaration of record types: table, owner, deletion and archiving columns, read and edit permissions and system fields |
| `FieldUseCases` | `back/kernel/src/application/fields/use_cases.rs` | Administration of fields and options (`settings.manage`), with event and audit |
| `CustomFieldWriter` / `CustomFieldReader` | `back/kernel/src/application/fields/{writer,reader}.rs` | Validated writing and batch reading of custom values |
| `Filter`, `RecordQuery`, `RecordQueryEngine` | `back/kernel/src/application/query` | Filter AST, resolution against the field registry and execution |
| SQL compiler | `back/kernel/src/infrastructure/query/compiler.rs` | Turns the resolved plan into SQL with `sqlx::QueryBuilder` |
| `ViewUseCases` | `back/kernel/src/application/views` | Saved views (filter, sort, columns) with visibility rules |

## Storage

- **`field_definitions`**: one row per field (`record_type`, unique `key`). `is_system = true` marks the standard fields, which point to a column of the module's table (`column_name`); the others are custom. `config` (JSON) stores limits, decimal places, currency, regex, default phone country, target record type and default value. Fields are never deleted: deactivating them (`is_active = false`) only hides them, and the values remain in the database.
- **`field_options`**: select options (stable and immutable `value`, `label`, `color`, `position`, `is_active`).
- **`custom_field_values`**: one value per record and field (key `(record_type, record_id, field_id)`), in typed columns: `value_text` (text, URL, email, phone in E.164, referenced user and record), `value_long_text`, `value_number` (`DECIMAL(24,6)`, number and currency), `value_date`, `value_datetime` (UTC), `value_bool` and `value_option_id`. There are `(field_id, column)` indexes for each type, so filtering and sorting by a custom field uses an index.
- **`custom_field_multi_values`**: multi-select, one row per checked option.
- **`saved_views`** and **`saved_view_user_defaults`**: saved views and each user's default view per record type.

Empty values are not stored: `null`, blank text or an empty list delete the row. So `is_empty` is simply "no row exists".

### Field types

| Type | Column | JSON input and output | Validation |
| --- | --- | --- | --- |
| `text` / `long_text` | `value_text` / `value_long_text` | string | trimmed, `max_length` (text up to 1024), `pattern` (regex, `text` only) |
| `number` / `currency` | `value_number` | number (numeric string is also accepted) | `min`, `max`, `decimals` (currency: 2 by default), `currency` |
| `date` / `datetime` | `value_date` / `value_datetime` | `YYYY-MM-DD` / RFC 3339 (output in UTC) | strict format |
| `select` / `multi_select` | `value_option_id` / multi-value table | option `value` / list of `value` | existing and active option |
| `checkbox` | `value_bool` | `true`/`false` | boolean only |
| `url`, `email`, `phone` | `value_text` | string | http(s) URL (without a scheme it becomes `https://`), lowercase email, phone normalized to E.164 with `phonenumber` (`default_country`, default BR) |
| `user` | `value_text` | user id | must be a member of the organization |
| `record` | `value_text` | record id | record of the `target_record_type` (core types only; custom objects link through relationships), visible to whoever writes it |
| `formula` | `result_type` column | output as the result type; never accepts writes | expression, type and cycles validated on save (see [Calculated fields](#calculated-fields)) |

Type changes are only allowed within the same storage group (`text`/`email`/`url`/`phone` and `number`/`currency`); anything else responds `incompatible_field_type`.

## System fields and seeding

Each module declares its system fields in the registry. They are written to `field_definitions` with `is_system = true`:

- **At installation**, within the same installer transaction (`insert_registration` calls `seed_system_fields`).
- **At boot** (`aphel serve`, `aphel worker` and `aphel migrate`, after the migrations), `sync_system_fields` writes whatever is missing. It plays the role of a "data migration": when a new module or a new system field appears in the code, the installation receives the rows. It is idempotent; for a row that already exists it only updates the type, column, required flag and configuration (label and position keep what the administrator chose).

For the interface, system and custom fields are the same. The administrator can only change the label and position of a system field. The type, the column and whether it is filterable or sortable always come from the registry in the code, never from the database.

### Computed system fields

A `SystemField` can be **computed** (`SystemField::computed(ComputedColumn)`, `kernel/application/fields/computed.rs`): instead of a column, it declares an expression. The kernel's generic variants are `DaysSince { column }`, `StageLimitReachedAt` and `StageLimitExceeded` (identifiers validated in the registry, SQL in `kernel/infrastructure/query/computed.rs`). A module that needs its own SQL **contributes the expression** with `ComputedColumn::expression(template)`: the template is an SQL snippet written in the module's code (it never comes from the user) in which `{record}` (`COMPUTED_RECORD_ALIAS`) is replaced by the alias of the record's row in each query; the kernel wraps it in parentheses and uses it like any other expression, without knowing the module's tables. Example: the `score` of the `scoring` module (`modules/scoring/src/lib.rs`, `primary_score`) reads `scoring_models`/`record_scores` and enters `contact`/`company` through `extend`. The resolver turns the field into `FieldTarget::Computed` (and `MetricTarget::Computed` in aggregations), and the compiler uses the expression in `WHERE`, `ORDER BY`, groupings, metrics, export and formulas. Computed fields are never searchable or required, are `read_only` in the `FieldSet`/`FieldAccessPolicy` (they can be hidden), and are excluded from import (`READ_ONLY_IMPORT_KEYS`), from formula aggregations and from the privacy columns (`SystemField::stored_column()` = `None`). The module returns the value in its own response (e.g. `DealResponse.days_in_stage`).

Deals use three (`modules/workflow/rotting.rs`): `days_in_stage` (whole days in UTC since `stage_entered_at`, which is always the last entry in `deal_stage_history`), `rotting_at` (`stage_entered_at + workflow_stages.rotting_days`, only open deals with a limit) and `is_rotting` (`rotting_at <= now`).

## Writing and reading

```rust
let changes = CustomFieldWriter::new(&ports)
    .apply(&mut tx, scope, &record, &custom_fields, WriteMode::Create)
    .await?;
```

- Validates everything before writing and returns `AppError::InvalidFields` with one item per field (`custom_fields.<key>`): unknown key, system field, inactive field, wrong type, limits, nonexistent or inactive option, missing required field, nonexistent or invisible reference, unique value already used.
- In `Create`, applies the `default_value`s and requires the required fields. In `Update`, ignores values equal to the current ones and only validates what was sent.
- Returns `Vec<FieldChange>` (`custom_fields.<key>`, old, new), ready to go into the module's audit diff.
- `CustomFieldReader::load(scope, record_type, &ids)` reads the values of N records in a single query and returns `custom_fields` per record, with `null` for active fields without a value.

## Filter AST

```json
{
  "op": "and",
  "conditions": [
    { "field": "name", "operator": "contains", "value": "ana" },
    { "op": "or", "conditions": [
      { "field": "temperature", "operator": "in", "value": ["hot", "warm"] },
      { "field": "created_at", "operator": "within_last_days", "value": 7 }
    ]}
  ]
}
```

- A node is a group (`op`: `and`/`or`, `conditions`), a condition (`field`, `operator`, `value`) or a related node (`{ "related": { "relationship": "<relation>", "filter": <node> } }`, "a visible related record exists that matches the filter"; the `relation.field` condition is the shortcut for a single condition). Unknown properties are rejected. At most 4 group levels, 50 conditions, 100 values per list and 3 sorts.
- Operators: `eq`, `neq`, `contains`, `not_contains`, `starts_with`, `gt`, `gte`, `lt`, `lte`, `between`, `in`, `not_in`, `is_empty`, `is_not_empty` and the relative date ones `today`, `yesterday`, `tomorrow`, `this_week`, `last_week`, `next_week`, `this_month`, `last_month`, `next_month`, `this_year`, `within_last_days`, `within_next_days`, `older_than_days` and `anniversary`. The schema (`GET /api/v1/fields/{record_type}/schema`) says which operators each field accepts and the value format of each one (`none`, `single`, `pair`, `list`, `days`).
- Relative dates use the organization's time zone (`organization_settings.timezone`); weeks start on Monday. In `datetime` fields, a `YYYY-MM-DD` value means the whole day in that time zone.
- `anniversary` (`days` value, from 0 to 3650) matches dates whose day and month fall between today and today + `days`, ignoring the year (birthdays and commemorative dates: `{ "field": "birth_date", "operator": "anniversary", "value": 7 }` is the next 7 days, including today). The window can cross the year boundary. The SQL computes the next anniversary with `DATE_ADD(date, INTERVAL n YEAR)`, so Feb 29 falls on Feb 28 in non-leap years. In `datetime` fields the local date uses the current offset of the organization's time zone.
- Negations (`neq`, `not_contains`, `not_in`, `is_empty`, checkbox `false`) include records without a value.
- In user fields, the value `"me"` becomes the current user (useful in saved views).
- In the listings' query string, `?filter=<JSON>&sort=-created_at,name&search=...&archived=exclude|include|only` (`RecordQueryParams` extractor in `back/kernel/src/api/dto/query.rs`).

## From filter to SQL

1. `RecordQueryEngine::query(scope, record_type, query)` finds the type in the registry, calls `scope.record_filter(&read_permission)?` (403 without permission) and loads the type's fields.
2. `resolve_query` validates the AST against the fields: nonexistent key, inactive field, non-filterable field, incompatible operator or invalid value become `InvalidFields` with the path (`filter.conditions.1.value`). The result is a typed `QueryPlan`. No user text becomes an SQL identifier.
3. The compiler (`build_query`) builds:
   - always `deleted_at IS NULL` and the archived filter;
   - the `Authorizer` scope: `own` → `owner = ?`; `team` → `owner = ?` or an owner who is a member of the user's teams (already expanded with sub-teams); a type without an owner column and a restricted scope → no records;
   - member access: a type that declares `member_access(table, record_column, user_column)` (validated as identifiers) adds `OR r.id IN (SELECT <record_column> FROM <table> WHERE <user_column> = ?)` to the `own` and `team` scopes, **for reads only**: the `RecordScopeFilter` carries `include_memberships`, turned on by `AccessScope::filter_for` only for `.read`/`.export` permissions. Editing, moving, archiving or deleting still requires being the owner (or having team/organization scope). In the `team` scope, only the user's own membership counts (not that of their teammates). Deals declare `member_access("deal_members", "deal_id", "user_id")` (deal team, `modules/deals/team`); the team-based pipeline restriction still applies to members. Everything that filters visibility through `push_record_scope_as` (listings, deals' `find`, global search — which switches the key to the index's `record_id` —, timeline, attachments and notes via `RecordAccessChecker`, dashboard, forecast, export) inherits the rule;
   - system field → condition on the column declared in the registry; custom field → `EXISTS (SELECT 1 FROM custom_field_values v WHERE … AND v.field_id = ? AND <predicate>)`; multi-select → `EXISTS` on `custom_field_multi_values`; negation → `NOT EXISTS`;
   - sorting by a custom field with a `LEFT JOIN` on that field's value (select sorts by the option's position), nulls always last and ties broken by `r.id`; in aggregations, groups by a select field (system or custom) also follow the options' position (`FIELD(..)` in the `ORDER BY`), not the alphabetical order of the value;
   - text search with `LIKE` on the `is_searchable` fields (system and custom);
   - all values with `push_bind`; `LIKE` with its own escaping (`ESCAPE '!'`). Tables and columns come only from the registry, validated as identifiers when the type is registered.
4. The executor runs the `COUNT(*)` and the paginated ids query and returns `Listing<Uuid>`. The module loads the records by their ids.

Performance reference (local MySQL 8, debug build): 10 thousand records with 5 custom fields, a 3-condition filter on custom fields plus sorting by one of them, count and page in about 170 ms.

## How a module registers itself

1. Declare the type in the registry. The native types (`company`, `contact`, `deal`, `activity`) live in `back/kernel/src/application/fields/builtin.rs`, used by `RecordTypeRegistry::builtin()`:

   The product catalog (`product`, table `products`, no owner column: only organization scope sees the catalog) is declared in `back/modules/catalog_items/src/mod.rs` and goes into the same `builtin_record_types()`.

```rust
EntityDefinition::new(RecordType::CONTACT, "people", Permission::CONTACTS_READ)
    .update_permission(Permission::CONTACTS_UPDATE)
    .owner_column("owner_id")
    .soft_delete_column("deleted_at")
    .archive_column("archived_at")
    .system_field(SystemField::new("name", "Nome completo", FieldType::Text, "full_name")?.searchable())
    .system_field(SystemField::new("first_name", "Nome", FieldType::Text, "first_name")?.searchable().required())
    .system_field(SystemField::new("owner_id", "Responsável", FieldType::User, "owner_id")?)
    .system_field(SystemField::new("created_at", "Criado em", FieldType::DateTime, "created_at")?)
```

   The table must have `id CHAR(36)`. Dates and times are stored in UTC.
2. In the use case's ports bundle, include a `CustomFieldPorts` (`MySqlCustomFieldPorts` in production) and a `RecordQueryPorts` (`MySqlRecordQueryPorts`).
3. Creation and editing: write the table row, call `CustomFieldWriter::new(..).with_record_values(&record_snapshot).apply(..)` with the `&mut tx` (the snapshot of the system fields after the change feeds the conditional rules), merge the changes into the audit diff and publish the event, all in the same transaction. Permanent deletion (trash, privacy, retention): `CustomFieldWriter::purge_record`, which goes through the pipeline's `purged` stage (`RecordPurged` hook, so modules can clean up what is theirs) and deletes the record's custom values and relationship links (`record_relations`). `CustomFieldWriter::clear_record` only deletes the values, without a hook (merging duplicates, where the record still exists as deleted). When uninstalling a module with `--purge`, `aphel_kernel::records::purge_record_type_data(tx, type)` deletes the custom values and links of its record types.
4. Listing: `RecordQueryParams` → `RecordQuery` → `RecordQueryEngine::query` → load the records by id in the returned order → `CustomFieldReader::load` to fill `custom_fields` in the DTO.
5. In tests, use the fakes from `tests/integration/support/fake_fields.rs` and the test record type from `tests/integration/support/test_records.rs` (table `test_widgets`, created only in the test).

## Saved views

`saved_views` stores name, type (`table`, `kanban`, `calendar`), filter, sort, columns and free-form `settings` (up to 16 KB). Visibility:

- `private`: only the owner sees and edits it.
- `team`: team members see it; creating it requires being on the team (or `settings.manage`); the owner or whoever has `settings.manage` edits it.
- `organization`: everyone sees it; creating and editing require `settings.manage`. Only these can be `is_default` (organization default; marking one unmarks the others).

Each user chooses their default view per record type (`PUT/DELETE /api/v1/views/{record_type}/{view_id}/default`). Filter, sort and columns are validated against the field registry on save.

## Tags

`tags` (unique name, color, `record_types` in JSON; an empty list applies to all) and `taggings(tag_id, record_type, record_id, created_by, created_at)` with key `(tag_id, record_type, record_id)`. Creating, editing and deleting tags requires `settings.manage`; tagging requires being able to change the record and publishes `record.tagged`/`record.untagged`. Types currently accepted: contact, company and deal.

In the query engine, `tags` is a virtual field (only when there is no real field with that key):

```json
{ "field": "tags", "operator": "in", "value": ["<tag_id>"] }
{ "field": "tags", "operator": "is_empty" }
```

Operators `in`, `not_in`, `is_empty`, `is_not_empty`, compiled as `[NOT] EXISTS` on `taggings` (table of the `tags` module, `back/modules/tagging`; the kernel compiler still reads the table directly). There is also the virtual `id` (`eq`, `neq`, `in`, `not_in`), used by bulk actions.

## Segments

A segment (`segments`) is a saved filter with a name and a record type; `GET /api/v1/segments/{id}/members` lists the members with `RecordQuery` and the count comes from `?counts=true`. As a condition in the AST:

```json
{ "field": "segment", "operator": "in_saved_filter", "value": "<segment_id>" }
```

The `RecordQueryEngine` expands the condition into the segment's filter (`resolve_query_with` + `SegmentResolution`). The kernel does not read the `segments` table: `MySqlRecordQueryExecutor::segment` asks for the filter from the `dyn SegmentSourceResolver` service (`kernel/application/query/segments.rs`), provided by the `segments` module (`back/modules/segments`, `app.provide`); without the service the condition responds `segment was not found`. A segment cannot reference another one. Queries outside the `RecordQueryEngine` that need to accept segments load the filters (`SegmentFilters`) and call `resolve_query_with(.., SegmentResolution::Expand(&segments))`: this is the case of the deals Kanban (`modules/deals/src/service.rs`), the approval policies and the scoring directory. Whatever still calls `resolve_query` directly (dashboard widgets) responds 422 for `in_saved_filter`.

## Company groups

`in_group` (value: a company id) matches the company and all its subsidiaries, at any level. It applies to `record` fields whose `target_record_type` is `company` (the `company_id` of people and deals, the `parent_company_id` of organizations and custom fields that point to a company) and to the virtual `id` of organizations:

```json
{ "field": "company_id", "operator": "in_group", "value": "<company_id>" }
```

In the query string: `filter[company_id][in_group]=<id>`. The resolver accepts the operator only on types that declare a hierarchy (`EntityDefinition::hierarchy`, declared by `organizations` with `parent_company_id`) and on `record` fields that point to them (`hierarchical_target`/`operators_for_definition`, which also feeds the field schema's `operators`); the compiler generates `column IN (WITH RECURSIVE ... SELECT id FROM record_group)` over the declared table and parent column, filtering the soft-delete column, with values always bound. The subquery does not apply the organizations' read scope: the filter says which group the record belongs to, and the scope of the queried type itself still applies. Details in [`company-hierarchy.md`](company-hierarchy.md).

## Form layouts and conditional rules

`form_layouts(record_type, context, layout)` with the `create`, `edit` and `view` contexts (`GET`/`PUT`/`DELETE /api/v1/layouts/{record_type}/{context}`; writing requires `settings.manage`). The layout is tabs → sections → columns (1 to 3) → fields:

```json
{ "tabs": [ { "id": "main", "label": "Principal", "sections": [
  { "id": "data", "label": "Dados", "columns": [
    { "fields": [ { "key": "company_id",
        "visible_when": { "field": "job_title", "operator": "is_not_empty" },
        "required_when": { "field": "job_title", "operator": "eq", "value": "CEO" } } ] } ] } ] } ] }
```

- Nonexistent, inactive or repeated fields are refused; required fields missing from the create layout go into the first column. A required field does not accept `visible_when`; the `view` layout only accepts `visible_when`; conditions only reference fields of the same layout.
- Conditions use the same filter AST, evaluated in memory over the form values (`kernel/application/layouts/conditions.rs` and `web/src/features/records/form/lib/conditions.ts`, with the same semantics as the engine: case-insensitive text, negations include empty values, relative dates in the organization's time zone). Operators or fields the form does not know (`in_saved_filter`, `tags`) evaluate as "does not match". Both evaluators run against the same cases in `back/tests/integration/fixtures/conditional_rules.json`.
- **Backend validation:** the `CustomFieldWriter` calls the `ConditionalRules` port (`LayoutConditionalRules` in production) with the record's system values after the change plus the merged custom ones. A field required by `required_when` and left empty becomes a 422 in `details.fields` (`<key>` or `custom_fields.<key>`). A field hidden by `visible_when` is not required. Creation uses the `create` layout; editing uses `edit`, or `create` when there is no `edit`.
- On the web, `RecordForm` renders the layout and evaluates the rules on every change; the side panel of the record page uses the `view` layout. The editor is at `/settings/layouts/:recordType`.

## Contributed fields in OpenAPI

A system field contributed by another module (`extend(..).system_field(..)`, e.g. `lifecycle_stage` from `leads` and `score` from `scoring` on people and organizations; details in [`record-pipeline.md`](record-pipeline.md#contributed-fields)) is not a property of the owning module's DTO, because the owning crate does not know who extends it. It goes into the JSON **flattened**, next to the module's own fields (`#[serde(flatten)] contributed: Map<String, Value>` in `ContactResponse`, `CompanyResponse` and in the create/edit bodies):

```json
{ "data": { "id": "…", "first_name": "Ana", "custom_fields": {}, "lifecycle_stage": "lead", "score": 42 } }
```

- **In OpenAPI** (`/api/openapi.json`) the DTO's schema lists only the owning module's properties and gets free-form `additionalProperties`; the contributed keys do not appear by name, type or example. On the web, `openapi-typescript` generates `ContactResponse & { [key: string]: unknown }` in `web/src/api/schema.d.ts`, so the code reads `contact.lifecycle_stage` with a cast or through the generic field block. The static schema also does not change according to the enabled modules: the same `openapi.json` applies to builds with and without the contributing module.
- **Runtime discovery**: `GET /api/v1/fields/{record_type}/schema` lists the contributed fields as system fields (`is_system: true`) with type, options, operators and the role's access, only when the contributing module is enabled (`SystemField::is_available`). It is the source for forms, columns and filters; client types should treat these keys as optional.
- **Writing**: the body accepts the keys at the same level (`PATCH /people/{id}` with `{ "lifecycle_stage": "customer" }`); `contributed_input` separates only the writable contributed ones and the rest of the unknown keys is ignored. Computed fields (`score`) are read-only. Validating the value is up to the contributing module (`RecordValidating` hook), and the error appears in `details.fields` with the flattened key (`lifecycle_stage`).
- **Permissions and redaction**: like any system field, the `record_redactor` removes the flattened key when the role hides it, and filtering/sorting by it follow the `FieldSet`.
- A new module that contributes a field must document the key, type and values in the module's own guide (and in the description of the type's route, if relevant), since OpenAPI does not describe them.

## Personal data and field permissions

Each field definition has `is_personal` and `is_sensitive` (sensitive implies personal), also editable on system fields; people' name, email, phones and address are personal from the start (`SystemField::personal()`). The flags are used by privacy ([`docs/privacy.md`](../privacy.md)) to locate, export and anonymize data subjects' data.

Permissions per field and role live in `field_permissions` (`hidden` or `read_only`; no row = editable), configured at `/settings/fields/:recordType/permissions`. The admin role and locked fields (required system fields, name/title, owner, dates and those that define pipeline/stage/status) are always editable. The rule is enforced on the server by `FieldAccessPolicy` (`kernel/application/fields/permissions`) and comes in through `FieldSet::for_scope`: hidden fields disappear from `active()`, so the schema, the `CustomFieldReader`, the export and the search already ignore them (in the export, the label of a `record` field only comes out if the exporter can read the related record through the `RecordQueryEngine` and its title field is not hidden; otherwise the cell carries only the id); filtering, sorting, view columns, segments and import mapping with a hidden field respond 422 `field_not_accessible`; writing to a hidden or read-only field responds 422 (`field_not_accessible`/`field_read_only`) in the `CustomFieldWriter` and in each module's use cases (keys sent via `provided_keys()`). Record responses remove the hidden keys (`record_redactor` in `kernel/api/routes/v1/field_permissions.rs`), timeline and audit diffs come out as `"[hidden]"` and the body of outgoing webhooks is restricted by the role of the endpoint's creator (`PublicEventRenderer::restrict`). A new module needs to pass `FieldSet::for_scope` in its queries, call the write check for system fields and redact the response.

## Calculated fields

A custom field of type `formula` stores an expression and a result type; the value is calculated by the server and **materialized** in `custom_field_values`, in the `result_type` column. That is why filtering, sorting, aggregating, exporting and searching work as with any other field: the query engine sees a `number`, `currency`, `date`, `text` or `checkbox` field.

```json
{ "key": "days_to_renewal", "label": "Dias até a renovação", "field_type": "formula",
  "config": { "expression": "days_until(renewal_date)", "result_type": "number", "decimals": 0 } }
```

- **Representation.** `field_definitions.field_type` holds `formula` and the `config` carries `expression` and `result_type` (`number`, `currency`, `date`, `text`, `boolean`), plus `decimals` (number/currency) and `currency` (currency). In memory the `FieldDefinition` has `field_type` = the result type and `config.formula = Some(FormulaSettings)`; `FieldDefinition::type_name()` returns `formula` for the API, events, audit and database. This way no type `match` needed a new branch, and the filter operators, sorting and the aggregator follow the result type.
- **Restrictions.** The type does not change to/from `formula` (`incompatible_field_type`); `expression`/`result_type` change through `PATCH` with the whole `config`. It does not accept `is_required`, `is_unique`, `default_value` or the other configuration properties.
- **Read-only.** `FieldSet::new` and `FieldAccessPolicy::access_for` give `read_only` access to every `formula` field (or `hidden`, if the role hides the field), including for the admin. The `CustomFieldWriter` responds 422 `field_read_only` to any write (API, import, automation, bulk actions) and does not apply creation defaults; import does not offer the field as a target.

### Language

Custom parser in `back/kernel/src/application/fields/formulas` (tokenizer → Pratt parser with a typed AST → type checker → evaluator), with no access to anything outside the record and the declared related records. Limits: 2000 characters, 32 nesting levels, 400 nodes, 20 arguments per function. Errors have a code and a position (`start`/`end` in characters, zero-based).

| Element | Examples |
| --- | --- |
| Record fields (system and custom, by key) | `amount`, `renewal_date`, `status` (select becomes the option label; date and time becomes the date in the organization's time zone). User, record and multi-select are not accepted |
| Literals | `10`, `2.5`, `"text"`, `'text'`, `true`, `false`, `null` |
| Operators | `+ - * /` (date ± number of days; date − date = days), `= != < <= > >=` (case-insensitive text), `and or not` |
| Dates | `today()`, `days_between(start, end)`, `days_until(date)`, `days_since(date)`, `add_days(date, days)` |
| Logic | `if(condition, then, else)`, `coalesce(a, b, ...)`, `is_empty(x)` |
| Text | `concat(a, ...)`, `upper(t)`, `lower(t)`, `len(t)` |
| Number | `round(n, places)`, `abs`, `floor`, `ceil`, `min(a, b, ...)`, `max(a, b, ...)` |
| Aggregation of related records | `count(relation)`, `sum(relation.field)`, `avg`, `min`, `max` (one argument) |

Aggregatable relations (`FormulaWorkspace::context`): `items` on deals (deal items: `total`, `quantity`, `unit_price`, `discount_value`), `has_many` relations from the type registry (e.g. `deals`/`people` on organizations) and configurable relationships (`RelationPath::Relationship`, e.g. `contracts` of a custom object). Only numeric fields go into `sum`/`avg`/`min`/`max`; deleted related records do not count (archived ones do). Null values propagate (`null + 1` is null), division by zero gives null and `count`/`sum` without related records give `0`. The result is rounded to `decimals`; text is truncated at 1024 characters.

### Dependencies and cycles

`compile_formula` returns the dependencies (fields, aggregations and use of `today`). On save, `formula_definition_violations` (called by `FieldUseCases` on creation and on `config` changes) validates the expression against the active fields, checks whether the inferred type fits the `result_type` and looks for a cycle in the formula graph (formula → formula of the same record and formula → aggregated formula of another type). A cycle responds 422 `formula_cycle` with the path in `details.fields[0].message` (`deal.a → deal.b → deal.a`).

### Recalculation

- **By event:** the `formulas.recalculate` consumer (`FormulaEventConsumer`, with the current registry via `CatalogEventConsumer`) accepts `contact.*`, `company.*`, `deal.*` (including `deal.item_*`), `activity.*`, `product.*`, `custom.*.*`, `record.linked`/`record.unlinked` and `field.created|updated|activated`. For the event's record it recalculates the type's formulas (in topological order, so a formula that depends on a formula uses the new value) and goes up to the "parent" records that aggregate that type (up to 3 levels), also using the old value of the owner column in the event's `changes` (a deal that changed company recalculates both). Deleting a record aggregated through a relationship enqueues the full recalculation of the parent type (the links have already been deleted).
- **In batch:** creating, changing or reactivating a `formula` field enqueues the `formulas.recalculate` job (`{ record_type, strict, depth }`), which goes through all records of the type in batches of 200 and then enqueues the types that aggregate this one.
- **Daily:** `formulas.daily` runs every hour (`0 5 * * * *`) and, if there are formulas and the local date (organization time zone) has changed since the last time (`formula_daily_runs`, single row), enqueues the full recalculation of all types with formulas. This is what keeps `today()`/`days_until`/`days_since` correct and fixes any value that fell behind.
- **No loop:** the recalculation writes directly through the `CustomFieldValueRepository` (without `CustomFieldWriter`, without a domain event and without audit), only when the value changes. So it does not generate `<type>.updated`, does not trigger automations or webhooks and does not come back to the consumer itself. The consequence is that a change in a calculated value does not appear in the timeline or trigger "field changed" triggers.
- **System scope.** Values are calculated once, with `AccessScope::system()`, and materialized: aggregations (`sum(deals.amount)`, `count(people)`) count **all** non-deleted related records, regardless of who reads the record, including deals outside the reader's `own`/`team` scope and aggregated fields that are hidden for their role. The value is one for the whole installation; there is no "per-reader formula". What protects the result is the permission of the **formula field itself**: it is a custom field like any other, so `FieldAccessPolicy` makes it `read_only` or `hidden` and the `record_redactor` removes the key in `custom_fields` from record responses (including embedded ones and webhook bodies) when the role hides it, after the `RecordSerializing` hook; the export also respects the hidden field; filtering, sorting and aggregating by it respond 422 `field_not_accessible`. Rule for whoever configures it: if the formula aggregates data that some roles cannot see, hide the formula field for those roles in the field permissions matrix.
- **Concurrency.** Writing a batch orders the writes by `(record_id, field_id)`, so two simultaneous recalculations lock the `custom_field_values` rows in the same order. If MySQL still responds with a deadlock (1213) or lock wait timeout (1205) — for example, against a user write on the same record —, `FormulaEngine::recalculate` re-evaluates and rewrites the batch up to 4 times, with increasing waits (50, 100, 150 ms). Once the attempts are exhausted, the error goes back to the job/consumer, which has its own retry; everything is idempotent.

### API

- `POST /api/v1/fields/formula/preview` (`settings.manage`): `{ record_type, expression, result_type, decimals?, record_id?, field_key? }` → `{ valid, errors: [{ code, message, start, end }], kind, dependencies, evaluated, value }`. With the `field_key` of an existing field, the preview uses the new expression instead of the saved one and detects cycles; with `record_id`, it calculates the value over the record (404 `record_not_found` if the record is not in the requester's read scope). Fields hidden from the requester become a `field_not_accessible` error.
- `GET /api/v1/fields/formula/catalog?record_type=` (`settings.manage`): usable fields (with their type), aggregatable relations with their numeric fields, and functions, for the editor.
- The editor is on the fields screen (`web/src/features/settings/fields/formula`), with validation while typing, insertion of fields/functions and a preview over a record.
