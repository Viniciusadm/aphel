# Lifecycle and lead inbox

The lifecycle stage (`lifecycle_stage`) is a `select` system field of people and organizations. The options come from the `field_options` table and can be configured on the fields screen; the stored value is the option's `value`. Defaults: `lead`, `qualified`, `opportunity`, `customer`, `former_customer`.

All of this is the `leads` module (`back/modules/leads`, session 31), which depends on `people` and `organizations` and is enabled by default (it can be disabled at `/api/v1/modules`):

| File | Contents |
| --- | --- |
| `mod.rs` | `LeadsModule`: manifest, `lifecycle_stage` field, validation hook, consumer, default service, routes, `lifecycle.manage` permission (granted to `admin` via `grant_to`) and `lead.*` events |
| `domain.rs` | transition rules, configuration, reasons, history and report (pure types) |
| `hooks.rs` | `Lead` type, `Lead*` hooks, `LeadConverter` service and the `validate_stage` handler |
| `service.rs` | use cases as functions over `&Ctx` (configuration, reasons, history, report, qualify, disqualify, convert) and the transitions consumer |
| `store.rs` | SQL (`SettingsStore`, `ReasonStore`, `HistoryStore`, `LeadRecords`) |
| `section.rs` | `lifecycle` package section |
| `http.rs` | routes and DTOs |
| `migrations/0001_create_leads.sql` | Owned lifecycle extension tables, history, configuration, reasons and disqualifications |

Web: `web/src/features/leads`.

## Contributed field

`people` and `organizations` do not own lifecycle data. Leads contributes `lifecycle_stage` through a registry transform and stores it in its own `leads_contact_extension` and `leads_company_extension` tables. The core reads, validates and writes these contributed fields through declared side-table metadata. The same fields are available to owner handlers, imports, bulk actions, automations and incoming webhooks without lifecycle implementation calls in people or organizations. Fresh schemas create these tables directly; there is no legacy-column copy/adoption migration. See [the P5.20 review](p5-leads.md).

## Validation and history (same transaction)

Every contact and company write goes through the record pipeline ([`record-pipeline.md`](record-pipeline.md)). The `leads::hooks::validate_stage` handler is registered by `leads` itself with priority 40 on the `RecordValidating` hook and, when the write changes `lifecycle_stage`:

1. rejects a value that is not an **active** option of the field for the record's type (422 `validation_error` with `details.fields[].field = "lifecycle_stage"`);
2. closes the open `lifecycle_stage_history` entry (`left_at`) and opens the new one (`from_stage`, `to_stage`, `entered_at`, actor), in the write's `&mut Tx`.

Why a hook and not a port in the contact and company use cases: the hook already runs inside the transaction on every write path (API, imports, automations, incoming webhooks, extensions through the host, email capture, consumers), receives the changes with old and new values and does not require `people`/`organizations` to call `leads`. With the module disabled the hook does not run (any value is accepted and there is no history). Clearing the stage only closes the open entry. Creating with a stage already opens the first entry.

Deleting a contact or company closes the open entry (`RecordDeleted` hook, `hooks::close_stage`). Restoring from the trash reopens the entry with the current stage (`RecordRestored` hook). Merging duplicates does not go through the guard; for that the module exposes `leads::service::record_stage_history(ctx, tx, record, from, to)` (records the transition if it changed), called within the same transaction.

## Automatic transitions

Organization configuration in `lifecycle_settings` (single row) (`GET`/`PUT /api/v1/lifecycle/settings`, administrative permission `lifecycle.manage`). Without a stored row, the defaults apply. Each rule has `enabled`, `target_stage`, `from_stages` and `from_empty`:

| Trigger | Events | Default target | Applies from (default) |
| --- | --- | --- | --- |
| `deal_created` | `deal.created`, `deal.reopened` | `opportunity` | `lead`, `qualified`, `former_customer`, no stage |
| `deal_won` | `deal.won` | `customer` | `lead`, `qualified`, `opportunity`, `former_customer`, no stage |
| `all_deals_lost` | `deal.lost`, `deal.deleted` | `qualified` | `opportunity` only |

No-downgrade rule: a transition only changes the record when the current stage is in `from_stages` (or it is empty and `from_empty` is true), and never when it is already at the target. That is why a customer who opens a new deal stays `customer`, and losing a deal only sends back to `qualified` those still in `opportunity`. `all_deals_lost` also requires the contact or company to have at least one lost deal and no open or won ones (deals in the trash do not count).

Execution: the `lifecycle.transitions` consumer (`CatalogEventConsumer`, current registry) with system scope (`AccessScope::system()`, all permissions, `system` user). It obtains commercial parties/outcomes through the deals SDK capability and calls the registered owner handler through `Ctx::update_record` with only `lifecycle_stage`, which publish `contact.updated`/`company.updated` with `changes` (automation triggers) and go through the guard (history). A target that is not an active option of that type, or a nonexistent record, is ignored with a warning in the log (no retry).

## Lead inbox

`/leads` in the web app lists people with `lifecycle_stage = lead` through `RecordListView` (prop `baseFilter`, combined with `and` with the user's filters and views). Actions:

- `POST /api/v1/leads/{contact_id}/qualify`: `lead` → `qualified`.
- `POST /api/v1/leads/{contact_id}/disqualify` (`reason_id`, `note`): from `lead` or `qualified` to the configured `disqualified_stage` (default: no stage). Writes `lead_disqualifications` (reason with the name copied, note, original stage, actor).
- `POST /api/v1/leads/{contact_id}/convert` (`pipeline_id`, `stage_id`, `title`, `amount`, `currency`): from `lead` or `qualified`. Creates the deal through the `dyn LeadConverter` service (see below) and then changes the stage to the target of the `deal_created` rule (default `opportunity`), even with the rule turned off.

Permissions: `people.update` on the contact (with the record scope and the field permission for `lifecycle_stage`) and, to convert, `deals.create`. Outside the expected stage: 409 `contact_not_lead`.

Each action writes the stage, `contact.updated` (with `changes`), `lead.qualified|disqualified|converted` (public catalog, contact resource) and the audit entry in the same transaction. Conversion takes two transactions: first the deal (`deal.created`), then the stage. If the second one fails, the deal stays created and the transitions consumer still moves the contact to `opportunity`.

## Module hooks and service

Other modules and plugins extend the lead inbox without editing `leads` (types in `leads::hooks`, re-exported in `crate::modules::leads`):

| Point | Kind | When | Argument |
| --- | --- | --- | --- |
| `LeadQualifying` | action (veto) | in the qualification transaction, before writing | `Lead` |
| `LeadDisqualifying` | action (veto) | in the disqualification transaction, before writing | `DisqualifyingLead { lead, reason, note }` |
| `LeadConverting` | filter | in `DealLeadConverter`, before creating the deal | `LeadConversion { lead, deal, company }`: `deal` holds the deal values (same body as `POST /deals`); `company` (default `None`) creates a company when the contact does not have one, and the deal is linked to it |
| `LeadConverted` | action | in the stage transaction, after writing | `ConvertedLead { lead, deal_id, company_id }` |
| `dyn LeadConverter` | service | `convert_lead` resolves it from the container | default `DealLeadConverter`: creates the company (optional) and the deal through the `RecordTypeHandler` of the `company` and `deal` types, without importing the deals code |

`Lead` carries an owner-produced scoped record snapshot, identity and precise version, with lifecycle helpers. An `Err` from an action or filter rolls back the operation. To replace the conversion: `app.provide::<dyn LeadConverter>(..)`; to wrap the default one: `app.decorate::<dyn LeadConverter>(|inner| ..)`.

Lead sources: the `lead_forms` module sends `lifecycle_stage = lead` when creating the contact (when the field exists and the form did not map another value), and email capture creates people in `lead`.

## Disqualification reasons

`lead_disqualification_reasons`, seeded at installation (`seed_disqualification_reasons`, called by `seed_defaults`). CRUD at `/api/v1/lead-disqualification-reasons` (list: `people.read`; write: `lifecycle.manage`), reordering at `PUT .../order`. Deleting a reason keeps the name in the disqualifications already recorded.

## Report

`GET /api/v1/lifecycle/report?record_type=contact|company&from=&to=` (inclusive UTC dates, default 90 days, maximum 731), with the read scope of `people.read`/`organizations.read`. Stages in the order of the active options (old values found in the history come at the end, marked as inactive). Per stage:

- `current`: records in the stage now (excluding archived ones);
- `entered`: records that entered the stage in the period;
- `converted`/`conversion_rate`: of those that entered, how many later reached any stage at a higher position (skipping a stage counts);
- `average_days`/`completed`: average time in the stage for the period's entries that have already left it.

For people, it includes the period's disqualifications by reason. Web: `/leads/report`.

## Configuration packages

`lifecycle` section (`leads::section::LifecycleSection`): `disqualification_reasons` (by key; creates, updates and reactivates, never deletes), `transitions` (replace the rules provided) and `disqualified_stage`. Stages are referenced by the option's `value`; the package does not create options (that belongs to the fields section).

## Web

- `web/src/features/leads`: `LeadsPage` (`/leads`), `LifecycleReportPage` (`/leads/report`), `settings/LifecycleSettingsPage` (`/settings/lifecycle`: rules and reasons), conversion and disqualification dialogs.
- "Lifecycle" tab on the contact and company detail pages (`useLifecycleTab`/`withLifecycleTab`) with the history from `GET /api/v1/lifecycle/history/{record_type}/{record_id}`.
