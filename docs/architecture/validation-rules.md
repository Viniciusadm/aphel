# Validation rules and stage guidance

Two organization settings that come in as **handlers of the `RecordValidating` hook** of the record pipeline ([`record-pipeline.md`](record-pipeline.md)) and therefore apply to every record write, whether it comes from the form, inline editing, the API, import, automations or bulk actions: all these paths use the same use cases, and the use cases for contact, company, deal (create, edit, move stage, win, lose, reopen), activity, product, quote and custom object call `fields().pipeline().validate(&mut tx, scope, &RecordWrite { .. })` inside the transaction, after writing the custom fields and before publishing events.

The two guards are registered by the `workflow` module (checklist, priority 20) and the `kernel` module (validation rule, priority 30, in `back/kernel/src/kernel_module.rs`, after approval and before lifecycle):

- `StageChecklistGuard` (`modules/workflow/hooks.rs`);
- `ValidationRuleGuard` (`kernel/application/fields/validation_rules/guard.rs`).

## Validation rules

Code: `kernel/application/fields/validation_rules` (model, ports, use cases, guard, evaluation and package section), `kernel/infrastructure/fields/validation_rules` (MySQL), API `/api/v1/validation-rules` (`kernel/api/routes/v1/validation_rules.rs`), web `web/src/features/settings/validation-rules` (route `/settings/validation-rules`). Table `validation_rules` (migration `2550`).

A rule has:

| Field | Meaning |
| --- | --- |
| `key` | Stable, unique key (snake_case), used by configuration packages. Generated from the name when omitted. Does not change. |
| `record_type` | A type that declares `EntityTraits::validation_rules()` (in the default build: `contact`, `company`, `deal`, `activity`, `product`, `quote`) or `custom.<key>`. Does not change. |
| `name` | Name for the administrator. |
| `expression` | Boolean formula (the same language as formula fields, `kernel/application/fields/formulas`) that describes the **error condition**: when the result is `true`, the write is rejected. A `false` or empty result lets it through. |
| `message` | Message shown to the user (free text written by the administrator, in pt-BR). |
| `field` | Field highlighted in the error (optional). |
| `applies_on` | `always`, `create` or `update`. |
| `is_active` | Deactivating keeps the rule stored without applying it. |
| `position` | Evaluation and display order within the record type. |

Example: `amount > 10000 and is_empty(source)` with the message "Provide the source for deals above 10 thousand" and `field = source`.

### Error

All active rules for the type and the moment are evaluated; if any fails, the API responds **422 `validation_rule_failed`** with `details.fields[]` containing one `{ field, message }` per violated rule. `field` is the key of the highlighted field (system field), `custom_fields.<key>` (custom field) or `record` when the rule does not highlight a field. On the web, `mapRecordApiError`/`fieldErrorMessage` (`features/records/form/lib/errors.ts`) show the rule's message on the highlighted field or, when there is no visible field, as a general form error; toasts (`errorMessage` in `components/feedback/notify.ts`) show the rules' messages.

### Evaluated state

The rule is evaluated against the record's **proposed state**:

- system fields: `RecordWrite.values` (snapshot after the write); on edit, system fields that are not part of the snapshot (computed ones, such as `days_in_stage`) come from the stored state;
- custom fields: read **inside the transaction** after the `CustomFieldWriter` writes;
- formula fields: recalculated in order over these values;
- aggregations over relationships (`sum(items.total)`, `count(tasks)`): evaluated against the **stored state** (child records do not change when the parent is written); on creation they are 0 (`count`/`sum`) or empty.

The rule is compiled (the formula engine's parser + analyzer) on save: it must result in a boolean and reference active fields of the type. If a field it uses is removed later, the rule no longer compiles and is ignored on write (with a warning in the log) until it is fixed.

### API

`settings.manage` for everything:

- `GET /validation-rules?record_type=` — list;
- `POST /validation-rules` — create (`validation_rule.created`);
- `GET|PATCH|DELETE /validation-rules/{id}` — read, update (`If-Match`, `validation_rule.updated`) and delete (`validation_rule.deleted`);
- `PUT /validation-rules/order` — `{ record_type, ids }` reorders (`validation_rule.reordered`);
- `POST /validation-rules/preview` — `{ record_type, expression, record_id? }` validates the condition and, with `record_id`, evaluates it against the record's stored state (`fails`).

Every mutation writes a domain event and audit in the same transaction. The `validation_rule.*` events are internal (outside `public_event_catalog`).

### Configuration package

Section `validation_rules` (`ValidationRulesSection`): a list of `{ key, record_type, name, expression, message, field?, applies_on, active }`, reconciled by key. Installing creates or updates (including the active state) and never deletes; an existing rule with a different `record_type` is an unresolvable conflict. The condition is not compiled on install (the fields may come in the same package); the rule above applies to conditions that do not compile.

## Stage guidance

Code: `modules/workflow/guidance`, `modules/workflow/guidance`, API in `api/routes/v1/stage_guidance.rs`, web in `web/src/features/workflow/guidance` (editor in the stage dialog and the "Stage guidance" panel in the deal detail). Tables `workflow_stage_guidance`, `workflow_stage_checklist_items` and `deal_checklist_progress` (migrations `2551`–`2552`).

- Each stage has markdown guidance and a checklist (items with a `key` that is stable within the stage, label, required or not, and order).
- Progress is per deal and item (who checked it and when). Checking/unchecking requires being able to edit the deal and publishes `deal.checklist_item_checked` / `deal.checklist_item_unchecked` with audit (`checklist.<key>` in the changes).
- **Required items of the current stage must be done to move forward**: going to a stage with a higher position in the same pipeline or winning the deal. Going back a stage, moving to another pipeline, losing and reopening do not require the checklist. Neither does creating the deal directly in a stage. The rejection is **422 `stage_checklist_incomplete`** with `details.fields[]` = `{ field: "checklist.<key>", message: <item label> }`. Stage field requirements (`modules/workflow/requirements.rs`, `stage_requirements_missing`) are still checked first, in the use case; the checklist is checked in the guard, inside the transaction. When a required empty field is hidden or read-only for whoever is moving the deal, the rejection changes to **422 `stage_requirements_inaccessible`** (same `details.fields`, with the message indicating the inaccessible field): the user cannot resolve it on their own and the interface should not open the fill-in form; ask someone who can see the field or adjust the field permissions. The requirement is never ignored.

### API

- `GET /pipeline-stages/{id}/guidance` — guidance and checklist (anyone who can read deals);
- `PUT /pipeline-stages/{id}/guidance` — `{ guidance?, items?: [{ id?, key?, label, required }] }` replaces the checklist in the given order (items not in the list are removed along with their progress), `settings.manage`, `If-Match`, event `pipeline.stage_guidance_updated`;
- `GET /deals/{id}/checklist` — guidance, items of the current stage and the deal's progress, with `required_pending`;
- `PUT /deals/{id}/checklist/{item_id}` — `{ done }`.

### Configuration package

Section `stage_guidance` (`StageGuidanceSection`): a list of `{ pipeline, stage, guidance, items: [{ key, label, required }] }` with pipeline and stage by key (from the package or the organization, such as `pipeline_renewals`). Installing writes the guidance and creates or updates items by key; it never deletes items or progress.
