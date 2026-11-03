# Configuration packages (`aphel-config`)

A configuration package describes **how an organization works**, without any business data: custom fields and their options, workflow with stages and stage rules, loss reasons, activity types, tags, form layouts (with conditional rules), organization views, the default dashboard and automations (as drafts). The same format is used to export an organization's configuration, install it in another one and distribute the business templates embedded in the binary (B2B, agency, freelancer, real estate, SaaS).

- JSON Schema: [`schema.json`](schema.json)
- Built-in templates: [`blueprints/templates/`](../../blueprints/templates) (one `<key>.json` file per template; the template key is the file name)
- Code: `back/kernel/src/application/config_packages` (engine and sections), `back/kernel/src/infrastructure/config_packages` (MySQL, built-in templates), `back/kernel/src/api/routes/v1/config_packages.rs`

## Format

```json
{
  "format": "aphel-config",
  "format_version": 1,
  "min_aphel_version": "0.1.0",
  "name": "Vendas B2B",
  "description": "Funil consultivo para vendas entre empresas",
  "author": "Aphel",
  "locale": "pt-BR",
  "sections": {
    "custom_objects": {
      "objects": [{ "key": "contract", "label_singular": "Contrato", "label_plural": "Contratos", "icon": "file-text" }],
      "relationships": [{ "key": "contracts", "inverse_key": "company", "from_type": "company", "to_type": "custom.contract",
                          "cardinality": "one_to_many", "from_label": "Contratos", "to_label": "Empresa",
                          "required": true, "on_delete": "restrict" }]
    },
    "fields": [
      { "record_type": "custom.contract", "key": "name", "label": "Nome", "type": "text", "required": true, "searchable": true },
      { "record_type": "company", "key": "segment", "label": "Segmento", "type": "select",
        "options": [{ "value": "industry", "label": "Indústria", "color": "#1f6feb" }] }
    ],
    "workflow": [
      { "key": "vendas", "name": "Vendas", "default": true, "stages": [
        { "key": "prospeccao", "name": "Prospecção", "kind": "open", "probability": 10 },
        { "key": "proposta", "name": "Proposta", "kind": "open", "probability": 50,
          "entry_required_fields": ["amount"] },
        { "key": "ganho", "name": "Ganho", "kind": "won", "probability": 100 },
        { "key": "perdido", "name": "Perdido", "kind": "lost", "probability": 0 }
      ] }
    ],
    "lost_reasons": [{ "key": "preco", "name": "Preço" }],
    "task_types": [{ "key": "visit", "name": "Visita", "icon": "map-pin", "color": "#16a34a" }],
    "tags": [{ "key": "investidor", "name": "Investidor", "color": "#f59e0b", "record_types": ["contact"] }],
    "layouts": [{ "record_type": "deal", "context": "create", "layout": { "tabs": [] } }],
    "views": [{ "key": "abertas", "record_type": "deal", "name": "Abertas",
                "filter": { "field": "status", "operator": "eq", "value": "open" } }],
    "dashboard": { "widgets": [{ "key": "deals_by_stage", "size": "large", "settings": { "pipeline": "vendas" } }],
                   "period": "month" }
  }
}
```

| Field | Rule |
| --- | --- |
| `format` | Always `aphel-config` |
| `format_version` | Format version; this Aphel reads up to 1 |
| `min_aphel_version` | Minimum Aphel version (`MAJOR.MINOR.PATCH`); a package newer than the binary is refused (`aphel_too_old`) |
| `name`, `description`, `author` | Metadata shown in the plan and in the history |
| `locale` | `pt-BR` or `en`: language of the package's labels |
| `sections` | One entry per section; a missing section is not touched. An unknown section is an error (`unknown_section`) |

### Stable keys

Items reference each other **by key, never by id**: fields and options (`key`/`value`, which already exist in Aphel), activity types (`key`), workflow, stages (unique within the pipeline), loss reasons, tags and views (unique per record type). Keys follow `snake_case` (up to 64 characters).

Pipelines, stages, loss reasons, tags and views had no key in the database: the `config_item_keys` table stores the key of each item from the first package installation that mentions it. While an item has no stored key, its key is **derived from the name** (`Prospecção` → `prospeccao`, with a `_2` suffix in case of repetition). That is why the templates use keys equal to the ones derived from the seeded items (`prospeccao`, `qualificacao`, `proposta`, `negociacao`, `ganho`, `perdido`…), to reuse them instead of duplicating them.

A pipeline with `"default": true` that does not find a pipeline with the same key reuses the **organization's default pipeline** (if it does not have a stored key yet). Installing a package never changes which pipeline is the default.

Custom objects and relationships already have their own key (the object's `key`, which becomes the `custom.<key>` type, and the relationship's `key`). The `custom_objects` section is installed **before** `fields`, and `fields`, `layouts` and `views` recognize the `custom.<key>` types declared in the same package even before they exist in the organization. A new object automatically gets the `name` title field, unless the package declares `name` for the type in `fields`. An existing relationship with different types, inverse key or cardinality becomes an unresolvable conflict (`relationship_incompatible`); labels, `required` and `on_delete` are updated. Inactive objects are not exported (nor the relationships that touch them).

References between sections are also by key: `entry_required_fields`/`exit_required_fields` mention deal field keys; layouts and views mention field keys and option values; the `deals_by_stage` widget mentions the pipeline in `settings.pipeline`.

## Validation

`POST /api/v1/config-packages/plan` validates before planning and returns `issues` with a path (`sections.fields.3.type`), code and message:

- structure and types (the JSON is read with `deny_unknown_fields`), format and Aphel version, language;
- valid keys without repetition, names, colors, icons, probabilities, field types, field configuration (`config`), options;
- cross references: a field mentioned by a stage, layout, view or conditional rule must exist **in the package or in the organization**; the same goes for a pipeline mentioned by the dashboard;
- `formula` fields (`type: "formula"` with `config.expression` and `config.result_type`): the expression is compiled against the package's and the organization's fields, checking syntax, result type and cycles (`invalid_formula`, `formula_cycle`); after installing, the types with a created or changed formula are recalculated by the `formulas.recalculate` job;
- each module's rules: a pipeline with `open`, `won` and `lost` stages; system field keys are reserved; layouts go through the same `validate_layout` as the screen; views through the same `validate_view_query`; limits (200 custom fields per type, 500 options, 50 workflow…).

Features that are off do not invalidate a package: a section that belongs to a disabled module is neither validated nor applied and comes back in `skipped_sections`; references to system fields of a disabled module (e.g. `mrr` from `line_items` in a layout, a view column or sort, or a stage's required fields) are removed before validating and come back in `skipped_fields` as `<record_type>.<field>`, and a view whose filter uses one is left out. Everything else in the package is installed normally.

## Installation plan

The plan compares package × organization and returns **one item per thing** (field, pipeline, stage, loss reason, activity type, tag, layout, view, dashboard), with an `id` (`<section>:<type>:<key>`), the action and the changes (`property`, `current`, `proposed`):

| Action | Meaning |
| --- | --- |
| `create` | Does not exist in the organization; will be created |
| `update` | Exists and differs; will be updated (a deactivated item is reactivated) |
| `unchanged` | Exists and is the same |
| `skip_conflict` | Exists in an incompatible way; **not applied without a decision** |

Conflicts (`conflict.code`, `conflict.resolvable`):

| Code | When | Can it be applied? |
| --- | --- | --- |
| `field_type_mismatch` | Field with the same key and a different type | Only if the type can change without loss (`text`/`email`/`url`/`phone`, `number`/`currency`) |
| `field_duplicate_values` | The package marks as unique a field whose values already repeat | No |
| `stage_kind_mismatch` | Stage with the same key and a different kind (`open`/`won`/`lost`) | No |
| `tag_name_taken` | Another tag with the same name already exists | No |
| `automation_record_type_mismatch` | Automation with the same key and a different `record_type` | No |
| `non_portable_reference` | Automation mentions ids that may not exist in the organization (fixed user, incoming webhook, uuid in a filter or value) | Yes (installs the draft as is) |
| `non_portable_reference` (view) | The filter of the view to create or change mentions ids (user, team, record, tag, stage…) or uses `in_saved_filter` | Yes (installs the view **without** those conditions) |
| `relationship_incompatible` | Relationship with the same key and different types, inverse key or cardinality | No |
| `relationship_key_collision` | The relationship's key (or inverse key) is already used by a system field, a custom field (from the organization or the package) or another relation of the same record type | No |

Guarantees: installation **never deletes anything** (options, stages, fields and organization items that the package does not mention stay as they are) and **never touches business records** (people, organizations, deals, tasks, field values). New stages come in the package's order; stages that only the organization has stay after the stage that preceded them.

## Applying

`POST /api/v1/config-packages/apply` receives the same package (or `template`), the plan's `fingerprint` and the conflict decisions (`{ "<item id>": "apply" | "skip" }`, default `skip`). The plan is rebuilt on the server: if the organization changed since the plan, the response is 409 `plan_outdated`. A decision for an item that is not a conflict, or `apply` on an unresolvable conflict, is 422 `invalid_decision`.

Everything runs in one transaction: the sections write in installation order, the installation goes to `config_package_installs` (whole package, summary, who and when), the `config_package.applied` event goes to the outbox and the audit log records one change per applied item. Reapplying the same package is idempotent: the plan is all `unchanged`, and the stored installation carries `summary.no_op = true` (the history shows that nothing changed).

## API

| Method and path | What it does |
| --- | --- |
| `GET /api/v1/config-packages/sections` | Available sections, in installation order |
| `POST /api/v1/config-packages/export` | Exports the organization's configuration (`sections`, `name`, `description`, `author`, `locale` optional) |
| `GET /api/v1/config-packages/templates`, `GET .../templates/{key}` | Built-in templates |
| `POST /api/v1/config-packages/plan` | Validates and plans (`package` or `template`); writes nothing |
| `POST /api/v1/config-packages/apply` | Applies the approved plan |
| `GET /api/v1/config-packages/installs`, `GET .../installs/{id}` | Installation history (the detail includes the applied package) |

All of them require `settings.manage`.

## Export

Exports only what is portable: **active** custom objects and the relationships between exported (or core) types, **active** custom fields (with active options), active workflow and stages, active loss reasons and activity types, all tags, saved layouts, views with `organization` visibility, the organization's default dashboard (when it exists) and the current draft of all automations (without secrets). Left out are team-based pipeline visibility, private and team views and any reference to a user or record.

Round trip: exporting from A, installing into a new organization B and exporting B produces the same sections.

## Default dashboard

The `dashboard` section stores the **organization's** default layout (`dashboard_default_layouts`). Anyone who has not saved their own layout yet sees this default (filtered by permissions); without it, the built-in default applies. Restoring the default on the dashboard goes back to the organization's one.

## Automations

The `automations` section (session 17) is installed last, after `custom_objects`, `fields`, `workflow`, `task_types` and `tags`. Each item carries `key` (the automation's own key, `^[a-z][a-z0-9_]{0,63}$`), `name`, `description`, `record_type` (`contact`, `company`, `deal`, `activity` or `custom.<key>` from the package or the organization), `trigger` and `definition` (the current draft, in the same format as the automations API):

```json
"automations": [
  { "key": "follow_up_proposta", "name": "Follow-up da proposta", "record_type": "deal",
    "trigger": { "type": "deal.stage_changed", "to_stage": "vendas.proposta" },
    "definition": { "start": "tarefa", "steps": [
      { "id": "tarefa", "type": "action", "action": "create_task",
        "config": { "title": "Ligar para o cliente", "due_in": "2d", "activity_type": "call" }, "next": "etiqueta" },
      { "id": "etiqueta", "type": "action", "action": "add_tag", "config": { "tag": "investidor" } }
    ] } }
]
```

Reference translations (export replaces the id with the key; installation does the reverse, using first the items installed by the package itself and then the organization's):

| In the organization | In the package |
| --- | --- |
| `config.tag_id` (`add_tag`/`remove_tag`) | `config.tag`: tag key (the same as in the `tags` section) |
| `config.stage_id` (`move_stage`) | `config.stage`: `<pipeline>.<stage>` (the same keys as in the `workflow` section) |
| `config.activity_type_id` (`create_task`) | `config.activity_type`: activity type key |
| `trigger.from_stage_id`/`to_stage_id` | `trigger.from_stage`/`to_stage`: `<pipeline>.<stage>` |
| `config.sequence_id` (`enroll_in_sequence`) | `config.sequence`: sequence key (from the package's or the organization's `sequences` section) |
| `trigger.segment_id` (`time.schedule`) | `trigger.segment`: `"<record_type>/<segment name>"` |
| `value` of `in_saved_filter` conditions (in any filter of the trigger or the definition) | `"<record_type>/<segment name>"` instead of the uuid |

Segments are not part of packages: `trigger.segment` and `in_saved_filter` conditions by key must point to a segment that already exists in the organization (otherwise validation reports `unknown_reference`); a uuid in `in_saved_filter` stays as is.

Validation checks for a unique key, name (1–140), `record_type`, known triggers and actions compatible with the type, the flow structure (`structure_violations`) and that every reference by key exists in the package or in the organization; at most 200 automations per package (and per installation). Secrets never go into the package: the export removes `signing_secret`/`signing_secret_configured` from `call_webhook` and validation refuses those fields (`secret_not_portable`); after installing, configure the secret on the screen.

A new automation is created as a **draft** (`draft`, version 0, created by whoever installs it). An existing automation with the same key only has its draft updated (`name`, `description`, `trigger`, `definition`): status, version and published version do not change, so an active automation keeps running the published version until someone publishes again.

Not portable: fixed users (`{ "type": "user", "user_id" }`), `incoming_webhook_id` and any uuid inside filters or values stay as they are. When the created or changed automation mentions a uuid that the current draft did not have, the item becomes a resolvable `non_portable_reference` conflict, listing the paths; applying installs the draft anyway and it must be reviewed before publishing (publishing validates the references).

## Lead distribution

The `assignment_rules` section (`modules/assignment/section.rs`) reconciles each rule by key: installing creates or updates (keeping the round-robin state and the absences of members who remain) and never deletes rules. Members and route users go by **email** and the team by **name**, never by id. Conflicts:

| Code | When | Can it be applied? |
| --- | --- | --- |
| `assignment_members_missing` | There are emails or teams in the package that do not exist in the organization | Yes (the missing ones are left out; a rule left without any member and with the `members` fallback is installed **inactive**) |
| `assignment_record_type_unavailable` | The rule's record type does not exist or does not accept distribution in the organization | No |

## Sequences

The `sequences` section (session 23) is installed after `email_templates`. Each item carries `key`, `name`, `description`, `is_active`, `window_start`/`window_end` (`HH:MM`, organization time zone), `daily_limit` and `steps` (`kind` = `auto_email`, `manual_email`, `call`, `linkedin`, `task` or `wait`, with `wait_days`, `subject`, `body_html` and `title` depending on the kind). An email step's template is referenced by `template_key` (key of an organization template or of the same package's `email_templates` section); the export replaces the id with the key. Installing creates or updates the sequence by key (steps are replaced in the package's order; a step in the same position and of the same kind keeps its id and metrics) and never deletes sequences or touches enrollments.

## Currencies

The `currencies` section (session 24) is the list of currencies enabled besides the base one (ISO 4217 codes, at most 30), for example `"currencies": ["USD", "EUR"]`. Installing only enables the missing ones (it never disables) and ignores the organization's base currency; exchange rates are not part of the package.

The `line_items` section carries the deal items configuration: `{ "amount_mode": "manual" | "items" }` (in `items` mode the deal amount is the sum of the items). Installing only changes the mode when it differs from the current one; omitting the property keeps the value.

## Price lists and discount limits

The `price_lists` section (session 25) carries the price lists that apply to all organizations (`applies_to = all`; segment or company-specific lists are not exported), by stable key: name, description, currency, validity (`valid_from`/`valid_to`), active, currency default and items referenced by **product code** (`product_code`, `unit_price`, optional `min_quantity`, default 1). Installing creates the missing lists and updates the existing ones by key (the application scope is kept); items are replaced when they change. The currency must be enabled (or come in the same package's `currencies` section), otherwise the item is in conflict; nonexistent product codes generate a resolvable conflict (applying ignores those prices). Products are never created or changed.

The `discount_limits` section carries the maximum discount per role (`role` = role key, `max_discount_percent` as text or `null` for no limit). A nonexistent role becomes an unresolvable conflict.

## Renewals

The `pipeline_renewals` section (session 24) is installed after `workflow` and `lost_reasons`. Each item configures the automatic renewal of a pipeline's won deals: `pipeline` (source pipeline key), `enabled`, `days_before` (0–365), `target_pipeline` and `target_stage` (keys of the pipeline and of an open, active stage where the renewal is created, from the package or the organization), `title_prefix` (up to 60 characters) and `owner` (`original` or `none`). Installing creates or updates the pipeline's configuration, with the installer as the author (the renewal runs with their permissions), and never creates or changes deals.

## New section

The `automations` section (session 17) is the most recent example; a new section is added like this:

1. Implement `ConfigSection<Tx>` (`modules/config_packages/src/application/section.rs`): `key()`, `export`, `validate` (returns `PackageIssue` with a path relative to the section), `plan` (one `PlanItem` per item, with `created`/`compared`/`conflicting`) and `apply` (writes with `&mut Tx` only the items for which `decisions.executes(&item)`; plan and apply must come from the same reconciliation function).
2. If the section references fields, implement `prune` to drop the references listed in `UnavailableFields` (fields of disabled modules) and return them as `field_reference(record_type, key)`.
3. References to other sections: read their content in the received `ConfigPackage` (e.g. `package_fields`) and what already exists in the organization; ids created in the same transaction are in `ApplyContext::resolve`/`resolved`.
4. Register the section in `builtin_sections` (`kernel/infrastructure/config_packages/ports.rs`), in the right position of the installation order, and add it to [`schema.json`](schema.json).
5. Check round trip, conflicts and idempotency on a disposable database (automated tests have been suspended since session 11).

## Versioning

Packages with `id` and `version` in the manifest can be compared with the installed version and updated with per-item conflict resolution: see [`docs/community/versioning.md`](../community/versioning.md).
