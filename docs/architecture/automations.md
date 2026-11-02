# Automation engine

Automations run **trigger → conditions/branches → actions** flows on any automatable record type (`contact`, `company`, `deal`, `activity` and `custom.<key>` custom objects), without code. The engine runs in the worker, driven by the domain events in the outbox, keeps the history of every run and protects itself against cycles and duplicates. The visual interface belongs to session 18; everything here is usable through the API (`/api/v1/automations`).

## Pieces

The engine is the optional `automations` plugin (`back/modules/automations`). Shared trigger/action/gateway types and helpers belong to `aphel-contract-automation` (`back/contracts/automation`). Native plugins import `AutomationBuilder` from that contract to use `app.trigger`/`app.action`; both methods publish typed contributions. Execution, concrete gateways and the operation projection remain in the automations plugin.

| Piece | Where it lives | What it does |
| --- | --- | --- |
| States (`AutomationStatus`, `RunStatus`, `StepStatus`, `StepKind`, `SkipReason`) | `back/contracts/automation/src/domain` (automation contract) | Pure types, no serde |
| `TriggerSpec`, `AutomationDefinition`, `StepSpec`/`StepBody` | `back/contracts/automation/src/application/definition.rs` (automation contract) | Shape of the trigger and of the step graph; structural validation of the graph |
| `AutomationTrigger` + `TriggerRegistry::builtin()` | `back/contracts/automation/src/application/triggers.rs` (automation contract) | Trigger contract and catalog (read from `app().contributions::<dyn AutomationTrigger>()`); `event_context`, `record_event`, `changes`, `single` helpers for implementers |
| `AutomationAction` + `ActionRegistry::builtin()` | `back/contracts/automation/src/application/actions.rs` (automation contract) | Action contract and catalog (read from `app().contributions::<dyn AutomationAction>()`); configuration validation helpers |
| Time-based trigger contract (`TimePlan`, `DueRule`, `parse_plan`), durations and waits | `back/contracts/automation/src/application/time` (automation contract) | Used by the engine and by modules with time-based triggers (e.g. `pipelines::rotting::DealRotting`) |
| `AutomationGateway` (port) | `back/contracts/automation/src/application/gateway.rs` (automation contract) | What actions and conditions can do in the rest of the system |
| `HandlerAutomationGateway` / `HandlerAutomationGatewayFactory` | `back/modules/automations/src/gateway_impl.rs` (automations plugin) | Gateway implementation (also used by the extension host and by email): records through the `RecordTypeHandler`; everything else goes to the `AutomationOperations` port |
| `ApiAutomationOperations` | `back/modules/automations/src/gateway.rs` (automations plugin) | Implementation of `AutomationOperations` reusing the modules' services |
| Templates (`Template`, `TemplateScope`, `TemplateRules`) | `back/contracts/automation/src/application/templates.rs` (automation contract) | Safe mini-evaluator for `{{ ... }}` and its validation |
| Generic triggers (`record.created/updated/deleted`, `manual`) and time-based ones (`time.date_relative`, `time.record_stale`, `time.schedule`) | `back/modules/automations/src/triggers.rs`, `time/triggers.rs` | Registered by the `automations` module |
| Generic actions (`update_fields`, `assign_owner`, `send_notification`, `send_email`, `create_related_record`) | `back/modules/automations/src/actions.rs` | Registered by the `automations` module |
| `ActionRunning` hook (`RunningAction`) | `back/modules/automations/src/hooks.rs` | Action (`app.on(ActionRunning, ..)`): veto before running each action (the error fails the step); filter (`app.filter(ActionRunning, ..)`): receives the filled-in `output` and can change the recorded result |
| Conditions (`TriggerEvaluation`, `record_part`) | `back/modules/automations/src/conditions.rs` | Evaluates `trigger.*` in memory and sends the rest of the filter to the `RecordQueryEngine` |
| `EngineInput::run` | `back/modules/automations/src/engine.rs` | Walks the graph (`Live` or `DryRun` mode), runs the `ActionRunning` hook, records each step and decides on failure/retry |
| `AutomationValidator` | `back/modules/automations/src/validation.rs` | Validation on save and on publish |
| Service (`service::create`, `update`, `publish`, `set_active`, `dry_run`, `run_now`, `runs`…) | `back/modules/automations/src/service.rs` | Use cases as `async fn(ctx: &Ctx<'_>, ..)` functions: CRUD, publish, pause/activate, dry run, manual run and history |
| `AutomationRuntime`, `AutomationTriggerConsumer` (`automations.trigger`) and `AutomationRunHandler` (`automation.run`) | `back/modules/automations/src/runtime.rs` | Matching events against active automations and running them in the worker; the `AutomationRuntime` holds the worker's `AppState`, the trigger/action registries and the gateway, and builds the `Ctx` for each run |
| Sweep, resume and cancellation (`TimeTriggerSweep`, `WaitingRunsResumer`, `WaitingRunsCanceller`) | `back/modules/automations/src/time` | `automations.time_sweep` sweep, `automations.resume_waiting` resume, `automations.cancel_waiting` consumer |
| Stores (`AutomationStore`, `AutomationRunStore`, `AutomationTimeStore`, `AutomationLocks`) | `back/modules/automations/src/store` | Concrete SQL over `automations`, `automation_versions`, `automation_runs`, `automation_run_steps`, `automation_time_cursors` (writes inside the transaction take `&mut MySqlTx`) and `GET_LOCK` |
| `AutomationClock` | `back/modules/automations/src/clock.rs` | Engine clock (`automation_clock()`), with the development offset |
| Routes and DTOs | `back/modules/automations/src/http.rs`, `dto.rs` | REST API (`automations` tag) |
| `incoming_webhook` trigger and `call_webhook` action | `back/modules/webhooks/src/automation.rs` | Registered by the `webhooks` module |

## Model (migration `modules/automations/migrations/0001_create_automations.sql`)

- **`automations`**: stable key (`key`, unique, used by configuration packages), name, description, `record_type` (immutable), the **draft**'s `trigger` and `definition`, `status` (`draft`, `active`, `paused`), `version` (latest published version; 0 = never published), `created_by`, `updated_by`.
- **`automation_versions`**: snapshot of each publication (name, type, trigger, definition, `trigger_type` for indexing, `published_by`, `published_at`). Runs always use the published version; editing the draft does not change what is running until it is published again.
- **`automation_runs`**: one row per (automation, trigger). `trigger_key` = `<event_id>:<record_id>` (or `manual:<uuid>`) with `UNIQUE (automation_id, trigger_key)`: this is the per-event idempotency. Stores version, trigger type, record, `status` (`running`, `waiting`, `succeeded`, `failed`, `skipped`), `depth`, `chain` (automations already in the causation chain), the `reason` for `skipped`, `error`, `attempts`, start and end.
- **`automation_run_steps`**: one step per row (`UNIQUE (run_id, step_id)`): kind, action, `input` (configuration with templates already resolved and **sensitive field values masked**), `condition_result`, chosen `next_step_id`, `output`, `status`, `error`, start and duration. Secrets never go here.
- **`domain_events.causation`** (JSON, `1704`): `{ run_id, automation_id, depth, chain }` on events generated by automation actions.
- **Waits** (`1830`): `automation_runs` gained `resume_at`, `waiting_step_id` and `trigger_context` (the trigger context, used to resume), with the indexes `(status, resume_at)` (resume) and `(record_type, record_id, status)` (cancellation per record). `status` accepts `cancelled` and `reason` gained `record_deleted`, `automation_paused` and `version_published`; steps have `kind` `wait`/`wait_until` and `status` `waiting`.
- **`automation_time_cursors`** (`1831`): one row per automation with a time-based trigger — version, `cursor_at` (how far the sweep has gone), `page_offset` (continuation of a full batch), `occurrence_at` (cron occurrence being processed) and `swept_at` (fair ordering between automations).
- **Sweep indexes** (`1832`): `deals` by `updated_at`, `stage_entered_at`, `last_activity_at` and `expected_close_date`; `contacts`, `companies` and `activities` by `updated_at`; `custom_records` by `(object_id, updated_at)`; `automation_versions (trigger_type)`. Date custom fields already had `custom_field_values_date_index`/`_datetime_index`.

New permission: `automations.manage` (administrative, no record scope; the admin role already has everything).

## Definition

```json
{
  "key": "follow_up_proposal",
  "name": "Follow up on sent proposal",
  "record_type": "deal",
  "trigger": { "type": "deal.stage_changed", "to_stage_id": "<id of the Proposal sent stage>" },
  "definition": {
    "start": "still_open",
    "steps": [
      {
        "id": "still_open",
        "type": "condition",
        "filter": {
          "op": "and",
          "conditions": [
            { "field": "status", "operator": "eq", "value": "open" },
            {
              "related": {
                "relationship": "activities",
                "none": true,
                "filter": {
                  "op": "and",
                  "conditions": [
                    { "field": "status", "operator": "eq", "value": "open" },
                    { "field": "title", "operator": "contains", "value": "Follow up on proposal" }
                  ]
                }
              }
            }
          ]
        },
        "next": "create_follow_up_task"
      },
      {
        "id": "create_follow_up_task",
        "type": "action",
        "action": "create_task",
        "config": {
          "title": "Follow up on proposal for {{record.title}}",
          "assignee": { "type": "record_owner" },
          "due_in": "2d"
        }
      }
    ]
  }
}
```

The created task is linked to the deal and shows up in its history (`activity.created` with the `automation` actor; the timeline returns `metadata.automation = { id, name }`). A second firing does not duplicate the task: the same event does not produce another run (`trigger_key`) and a new event falls into the "no open follow-up task" condition.

### Triggers (`trigger.type`)

| Trigger | Types | Events | Options |
| --- | --- | --- | --- |
| `record.created` | all | `<type>.created` | — |
| `record.updated` | all | `<type>.updated` (+ on deals `deal.stage_changed`, `deal.pipeline_changed`, `deal.amount_changed` (`amount`, `currency`, `amount_base`, `exchange_rate`), `deal.owner_changed` (`owner_id`) and `deal.expected_close_date_changed` (`expected_close_date`), whose fields are not repeated in `deal.updated`; the changes come from each event's `changes: [{ field, old, new }]`) | `fields`: only when one of the fields changed |
| `record.deleted` | all | `<type>.deleted` | — (the record is no longer available; conditions on it are false) |
| `deal.stage_changed` | `deal` | `deal.stage_changed` | `from_stage_id`, `to_stage_id` |
| `deal.won`, `deal.lost` | `deal` | same | — |
| `activity.completed` | all | `activity.completed` | on `activity`, the activity itself; on other types, the linked records (`record_type`/`record_id` and `links`) |
| `note.created` | all except `activity` | `note.created` | fires for the note's record |
| `manual` | all | none | only via `POST /automations/{id}/run` |
| `incoming_webhook` | all | `incoming_webhook.processed` (published by the session 12 receiver for each created/updated record) | `incoming_webhook_id` (required); the received body (up to 16 KB) goes into `trigger.payload.body` |
| `time.date_relative` | all | sweep | `field` (date or date-time, system or custom), `offset` (`-3d`, `+2h`; default `0d`), `at` (`HH:MM` in the organization's time zone, only for date fields) |
| `time.record_stale` | all (`same_stage` and `no_activity` only where there is `stage_entered_at`/`last_activity_at`, currently `deal`) | sweep | `mode` (`no_update`, `no_activity`, `same_stage`), `days` (1–3650) |
| `time.deal_rotting` | `deal` | sweep | none: uses the stage's `rotting_days` ("Deal stuck in stage") |
| `time.schedule` | all | sweep | `cron` (with seconds, organization time zone, occurrences at least 15 min apart), `filter` (list AST), `segment_id` |
| `email.received`, `email.sent`, `email.opened`, `email.clicked`, `email.bounced` | `contact`, `company`, `deal`, `custom.<key>` | same (one publication per record linked to the conversation) | — |

### Time-based triggers

They do not come from events: the scheduled job `automations.time_sweep` (every minute, `max_attempts = 1`) lists the active automations whose published version has a `time.*` trigger (up to 50 per sweep, from the least recently swept to the most recent) and, for each one, queries the records **through the `RecordQueryEngine` with the publisher's scope** (AST filter built by the engine or taken from the definition, never SQL with user text), in batches of 100 and at most 1000 records per sweep. Whatever is left continues in the next sweep (`page_offset`).

- **Window.** Each automation keeps a cursor; the sweep covers the instants `(cursor, now]` and then advances the cursor. On a version's first sweep the cursor starts at `published_at`: **nothing that was already due before publication fires** (just like events prior to publication), except for `time.deal_rotting` (catch-up below). A new version starts over from its own `published_at`.
- **`time.date_relative`**: the instant is `field value + offset` (date fields at midnight in the organization's time zone, or at `at`). The window becomes `field >= start - offset` and `field <= end - offset` (dates converted to whole days).
- **`time.record_stale`**: the same mechanism over `updated_at` (`no_update`), `stage_entered_at` (`same_stage`) or `last_activity_at` (`no_activity`, with `created_at` when the record never had an activity), with `offset = days`. When the record changes, the field value changes and the old staleness disappears; a new staleness only becomes due `days` after the last change.
- **`time.deal_rotting`**: the same mechanism over the computed field `rotting_at` (`stage_entered_at + rotting_days` of the stage, only for open deals in a stage with a limit), with `offset = 0`. Deduplication uses `stage_entered_at` (`DueRule.key_field`), not `rotting_at`: it fires **once per stage entry**, even if the stage limit changes later. Rule and fields in `modules/pipelines/rotting.rs`.
- **`time.deal_rotting` catch-up.** When the cursor of a published version is initialized (its first sweep), the sweep also fires for all deals that are already rotting (`pipelines::rotting::rotting_catch_up_filter(None)`, `is_rotting = true`), and the `automations.rotting_catch_up` consumer does the same, only for the stage, when it receives `pipeline_stage.rotting_tightened` (published by the `pipelines` module when a stage's `rotting_days` is set or reduced; `rotting_catch_up_filter(Some(stage_id))`). The key is the same as the window's (`time:<stage_entered_at>:<record>`), so anything that already fired for that stage entry does not fire again. The catch-up walks through all pages at once (outside the sweep's 1000-record limit).
- **`time.schedule`**: on each cron occurrence (only the most recent one, if the worker was stopped), every record matching `filter` and the segment fires.
- **Once per reason.** The sweep creates the `automation_runs` row before enqueuing `automation.run`, with a deterministic `trigger_key`: `time:<field value>:<record>` (relative date and stale; in `time.deal_rotting` the value is `stage_entered_at`) or `time:<occurrence>:<record>` (scheduled). The `UNIQUE (automation_id, trigger_key)` prevents a second firing for the same reason, even with overlapping or repeated sweeps.
- The trigger context goes into `trigger.payload`: `{ field, value, due_at, fallback }` or `{ occurrence_at, cron }`, with `trigger.occurred_at` = the due instant and the `system` actor. Conditions on the record (for example `status = open` for "stale deal") go in a `condition` step, as with any trigger.

### Waits and sequences

```json
{ "id": "wait_2d", "type": "wait", "duration": "2d", "recheck": { "field": "status", "operator": "eq", "value": "open" }, "next": "notify" }
{ "id": "day_before", "type": "wait", "until_field": "expected_close_date", "offset": "-1d", "next": "notify" }
{ "id": "until_won", "type": "wait_until", "filter": { "field": "status", "operator": "eq", "value": "won" }, "timeout": "7d", "next": "won", "timeout_next": "expired" }
```

- **`wait`**: exactly one of `duration` (`30m`, `2d`, `1w`, up to 3650 days) or `until_field` (a date/date-time field of the type, with an optional `offset`). The run becomes `waiting` with `resume_at` and `waiting_step_id`, and the step records `status = waiting` with `output.resume_at`. A date already in the past continues immediately; an empty field ends the run successfully at that step (`output.reason = field_empty`).
- **`recheck`** (same AST as conditions): evaluated again on resume; false ends the run successfully at the step (`condition_result = false`), like a false `condition`.
- **`wait_until`**: evaluates `filter` immediately; match → `next`. Otherwise, it waits and re-evaluates every 10 minutes (`resume_at = min(now + 10 min, deadline)`); past the `timeout` → `timeout_next` (absent = end). The deadline is stored in `output.deadline`.
- **Resume**: the `automations.resume_waiting` job (every minute) picks up due `waiting` runs of active automations with `SELECT … FOR UPDATE SKIP LOCKED`, switches them to `running` and enqueues `automation.run` in the same transaction (never resumes twice). The job reloads the record and the current scope of whoever published the version, reuses the steps already completed and continues from the waiting step. An `automation.run` that finds the run still `waiting` does nothing.
- **Versions**: the run continues on the version it started on.
- **Dry run**: the wait step shows up as `planned` with the computed `resume_at` and the path continues as if the wait had elapsed (the `recheck` is evaluated now); for a `wait_until` that does not match now, the dry run follows `timeout_next`.

### Cancellation

`definition.options` (optional): `{ "cancel_waiting_on_pause": true, "cancel_waiting_on_publish": false }`.

- **Record deleted**: always cancels (`automations.cancel_waiting` consumer of `<type>.deleted`, reason `record_deleted`).
- **Pause**: cancels the waiting runs (`automation_paused`), unless the published version has `cancel_waiting_on_pause: false`; in that case they stay waiting and resume when the automation is reactivated.
- **Publish** a new version: keeps the waiting runs on the old version; with `cancel_waiting_on_publish: true` on the new version, cancels them (`version_published`).
- **Deleting** the automation deletes its runs along with it.

Cancelled runs stay `cancelled` with `reason` and `finished_at`; the waiting step becomes `skipped`.

### Clock

The engine uses the `Clock` contract (`kernel/application/security`, the same one the other modules use) with the `AutomationClock` implementation (`modules/automations/clock.rs`, a single instance read once per process in `automation_clock()`): the system clock and, **only in development builds** (`debug_assertions`), an optional offset in `AUTOMATIONS_DEV_CLOCK_OFFSET` (for example `+2d`) to test sweeps and waits without waiting days. It applies to the engine (deadlines, `resume_at`, windows, `{{now}}`); relative filters of the `RecordQueryEngine` (`within_last_days` etc.) stay on the real clock. The job queue, the scheduler, session and token expiration and computed fields (`days_in_stage`, `is_rotting`) also stay on the database's real clock; the reason why and how to test cases that depend on them are in the "Development clock" note in [`docs/configuration.md`](../configuration.md#worker-and-scheduler).

### Conditions and branches

- **`condition`** (`filter`, `next`): the filter is the **same AST as the lists** (`FilterNode`). If it is false, the run ends successfully at that step.
- **`branch`** (`cases[] { key, filter, next }`, `default`): follows the first true case, otherwise `default` (absent = end). Yes/no is one case + `default`.
- `trigger.*` fields are evaluated in memory against the event (same operator rules as conditional layouts): `trigger.previous.<field>` (value before the change), `trigger.changed_fields` (use `eq`/`in`), `trigger.payload.*`, `trigger.event_type`, `trigger.actor.type|id`, `trigger.occurred_at`. The rest of the filter becomes a `RecordQueryEngine` query with `id = <record>` (includes archived), with the publisher's read scope and field permissions, so custom fields, tags, segments, relative dates and related records (`company.industry`, `related` node) work just like in lists.
- The `related` node gained `"none": true` (`NOT EXISTS`), also available in lists: "there is no related record matching the filter".

### Actions (`AutomationAction`)

| Action | Types | Required permission | Retries on transient failure |
| --- | --- | --- | --- |
| `create_task` (`title`, `description`, `activity_type_id` — default `task`, `assignee` — default record owner, `due_in` or `due_at`, `priority`, `link_to_record`) | all | `activities.create` | no |
| `update_fields` (`values`: system or custom fields, `null` clears) | all except `activity` | `<type>.update` | yes |
| `assign_owner` (`owner`) | all except `activity` | `<type>.update` | yes |
| `add_tag` / `remove_tag` (`tag_id`) | all except `activity` | `<type>.update` | yes |
| `move_stage` (`stage_id`) | `deal` | `deals.update` | yes |
| `create_note` (`body`) | all (on activities, on the linked record) | `notes.create` | no |
| `send_notification` (`recipients`, `title`, `body`) | all | — | no |
| `send_email` (`recipients`, `subject`, `body`) | all | — | no |
| `call_webhook` (`url`, `body`, `signing_secret`, `retry_safe`) | all | `settings.manage` | only with `retry_safe: true` |
| `create_related_record` (`record_type`, `values`, `relationship`) | all | `<target>.create` (+ `<type>.update` to link) | no |
| `send_owner_email` (`to`: an email field of the record or addresses, default `email`; `template_id` or `subject` + `body`; `tracking`) | `contact`, `company`, `deal`, `custom.<key>` | `emails.send` and `<type>.read` | no |

Users (`assignee`, `owner`, `recipients[]`) are `{ "type": "record_owner" }`, `{ "type": "user", "user_id" }` or `{ "type": "field", "field": "<user field>" }`. Actions go through the modules' use cases, so validation, field permissions, stage requirements, events, audit, notifications, search and outgoing webhooks work as they do for a change made through the interface.

Notification and email do not call the mailer: the action publishes `automation.message` and the `notifications.events` consumer creates the notifications (`automation_message`, in-app only; `automation_email`, email only, sent by the `notification.email` job according to preferences). `call_webhook` uses the same SSRF protection as outgoing webhooks and signs `X-Aphel-Signature` (`t=<ts>,v1=<hmac>`) when there is a `signing_secret`; the secret is write-only: it goes to the `SecretVault` (`owner_type = automation`, `name = <step_id>.signing_secret`) and the definition only stores `signing_secret_configured`. Sending `null` removes the secret.

### Dynamic values

Configuration strings accept `{{ ... }}`:

- `{{record.<field>}}`, `{{record.custom.<key>}}` (alias for `custom_fields`), `{{record.stage.name}}`, `{{record.owner.display_name}}`, `{{record.title}}` (uses `name` when the type has no `title`);
- `{{trigger.previous.stage_id}}`, `{{trigger.changed_fields}}`, `{{trigger.payload.<path>}}`, `{{trigger.actor.id}}`;
- `{{automation.id|key|name}}`, `{{run.id|depth}}`;
- `{{now}}`, `{{now + 2d}}`, `{{now - 30m}}` (RFC 3339) and `{{today + 1w}}` (`YYYY-MM-DD` in the organization's time zone); units `m`, `h`, `d`, `w`, up to 3650 days.

There is no arbitrary code: only paths and date offsets. A template that is a single expression returns the original JSON value (number, object…); mixed with text it becomes text. Missing paths become empty. On save, the validator rejects unknown roots and fields (`record.custom.x` must be a custom field of the type). In the step log, values coming from `is_sensitive` fields (and from `trigger.payload`) show up as `[redacted]`.

### Validation

On create, edit and publish: automatable, existing type; a trigger from the catalog that supports the type and has valid options (stages and webhook exist); graph with at most 50 steps, unique ids, existing `start`, no cycles and no orphan steps, valid branch paths; compilable filters (fields exist and are visible); known actions that support the type; valid configuration (tag, stage, activity type and users exist); resolvable templates. Time-based triggers: field exists and is a date, valid `offset`/`at`/`days`, valid (same parser as the scheduler) and sufficiently spaced cron, filter compiles and segment exists for the type. Waits: valid duration, only one of `duration`/`until_field`, a date field of the type, valid `timeout` and compilable `recheck`/`filter`. On publish, additionally: at least one action and **all the permissions required by the actions** in the publisher's scope (`422 invalid_automation` with the paths in `details.fields`).

## Execution

1. The `automations.trigger` consumer receives every event that some trigger listens to, computes the candidate records and looks up the **active** automations whose published version has that type and trigger (`automation_versions_trigger_index`). For each one whose options match, it enqueues `automation.run` with `unique_key = automation:<automation_id>:<event_id>:<record_id>` and the trigger context (`previous`, `changed_fields`, payload, actor) plus the event's `causation`.
2. The job creates (or finds again) the `automation_runs` row by `trigger_key` — finished runs do not run again — and takes the named lock `GET_LOCK('aphel_auto_<hash(automation, record)>')`: two runs of the same automation on the same record never run in parallel (without the lock within 10 s, the job goes back to the queue).
3. Safeguards, evaluated only on the first attempt; when one trips, the run ends `skipped` with the reason:
   - `automation_inactive`: paused after enqueuing;
   - `max_depth`: causation chain longer than 5 (`depth` = event depth + 1);
   - `causation_cycle`: the automation itself is already in `chain` (A → B → A stops on the second pass through A; an automation that reacts to its own change also stops);
   - `rate_limited`: more than 20 non-skipped runs of the same automation on the same record within 60 minutes.
   For the three cycle reasons, whoever created the automation receives the `automation_skipped` notification (at most one per automation and reason per hour), via the `automation.run_skipped` event (`Actor::System`).
4. The run uses the scope **of whoever published the version**, reloaded on every run (`build_access_scope` + `AccessScope::through_automation`): current role, teams and permissions. Deactivated user → `skipped` with `publisher_unavailable`. The actor for events and audit is `Actor::Automation(<automation_id>)` and every event generated by the actions carries `causation = { run_id, automation_id, depth, chain + [automation] }` (via `CausedEventPublisher`). Nonexistent record → `record_not_found`; existing but outside the publisher's scope → `record_not_accessible`.
5. The engine walks the graph, recording each step. Before each action, it checks the required permissions again (without them the step fails with `permission_denied`).
6. Failures: a `retry_safe` (idempotent) action that fails due to a transient error (persistence, network, non-2xx HTTP from the webhook) returns an error to the job, which retries (up to 5 attempts, queue backoff). On the new attempt, already completed steps are **reused** (not repeated) and the run continues from the step that failed. Non-idempotent actions or validation errors fail the step and the run (`failed`, no retry). On the last attempt the run ends `failed` with the error.

### Dry run and manual run

- `POST /automations/{id}/dry-run { record_id, use_draft?, changed_fields?, previous? }`: runs the published version (or the draft) against an existing record, with the requester's permissions. Conditions and branches are actually evaluated; each action returns `status: planned` with the resolved (masked) input and details such as the computed assignee and due date. Nothing is written.
- `POST /automations/{id}/run { record_id }`: only for active automations with a `manual` trigger; creates the run and enqueues the job.

## API

| Route | What it does |
| --- | --- |
| `GET`/`POST /automations` | Lists (filters `status`, `record_type`, `search`; each item with `stats: { last_run_at, runs_30d, failed_30d, waiting }`) and creates a draft |
| `GET /automations/catalog` | Triggers and actions with supported types and JSON Schema for the options, template roots, `trigger.*` paths and limits |
| `GET`/`PATCH`/`DELETE /automations/{id}` | Detail, draft editing (`If-Match`) and deletion (with versions, runs and secrets) |
| `POST /automations/{id}/publish` | Validates, creates the next version and activates (`If-Match`) |
| `POST /automations/{id}/pause`, `/activate` | Pauses or reactivates the latest published version |
| `POST /automations/{id}/dry-run`, `/run` | Side-effect-free test and manual run |
| `GET /automations/{id}/versions` | Published versions |
| `GET /automations/{id}/runs`, `/runs/{run_id}` | History (filters `status` — including `waiting` and `cancelled` — and `record_id`; runs with `resume_at` and `waiting_step_id`) and the steps of a run |

Everything requires `automations.manage`. Mutations publish `automation.created|updated|published|paused|activated|deleted` with audit (record `automation`). Error codes in [`docs/api/errors.md`](../api/errors.md).

## How to register a trigger

Implement `AutomationTrigger` (kernel contract) and register it in the owning module's `register` with `app.trigger(Arc::new(..))` or `app.triggers(..)`; `TriggerRegistry::builtin()` reads the triggers of the enabled modules:

- a stable `key` (it is the `trigger.type` stored in versions and indexed in `automation_versions.trigger_type`), `label`, `description`, `options_schema` (JSON Schema shown in the catalog, with `time_based` for time-based triggers);
- `supports(record_type)`, `listens_to(event_type)` and `candidates(event)`, which returns the records (`RecordRef`) the event fires for and the `TriggerContext` (use `event_context` and fill in `changed_fields`/`previous` when it makes sense);
- optional: `accepts(spec, candidate)` for the options, `validate(spec, fields)`, `references(spec)` (ids checked by the gateway), `record_filters(spec)` (filters compiled by the gateway during validation, with the `trigger.*` paths), `is_manual` and `is_time_based`.

Time-based triggers do not listen to events (`listens_to` false); the sweep logic lives in `modules/automations/time/sweep.rs` and each one's plan in `kernel/application/automations/time/triggers.rs` (`parse_plan`).

A new module gets the generic triggers automatically if it publishes `<type>.created|updated|deleted` with `record` and, on updates, `changes: [{ field, old, new }]`, and if the type is automatable: the owner declares `EntityTraits::automatable()` in the type definition (`is_automatable` reads the capability from the `App` registry; `custom.*` custom objects are always automatable).

The `create_task` action uses the `TaskOperations` service (`kernel/application/automations/operations.rs`), provided by the `activities` module (`modules/activities/operations.rs`), which resolves the default activity type and creates the activity through the module's use case. Without the module, the action fails with "tasks are not available in this installation". In templates (`{{record.<key>}}`), besides the type's active fields, `id`, `title`, `name`, `custom`, `created_at`, `updated_at` and the embedded `<base>` object of every `<base>_id` field of the type (e.g. `stage`, `owner`, `contact`, `company`) are available.

## How to register an action

Implement `AutomationAction` (kernel contract) and register it in the owning module's `register` with `app.action(Arc::new(..))`; `ActionRegistry::builtin()` reads the actions of the enabled modules:

- `key`, `label`, `description`, `config_schema`, `supports(record_type)`;
- `required_permissions(record_type, config)`: checked on publish and before each run;
- `retry_safe(config)`: `true` only if repeating the action does not duplicate its effect;
- `secret_fields()`: write-only configuration keys, stored in the vault (read them with `context.gateway.reveal_secret(scope, &context.secret_key(field))`);
- `validate` (with `ValidationContext`; use `context.violation(field, message)` for the path), `describe` (what the dry run shows) and `execute` (receives the configuration with templates already resolved and returns the `output` recorded on the step).

If the action needs something new from the system, add a method to the `AutomationGateway` port. If it is a generic record read or write, implement it in `HandlerAutomationGateway` through the type's handler; otherwise, add the method to `AutomationOperations` and implement it in `ApiAutomationOperations` reusing the module's use case — never direct SQL.

## Guarantees

- **Idempotency**: an event fires at most one run per (automation, record) (`UNIQUE (automation_id, trigger_key)` + the job's `unique_key`); time-based triggers, one per reason (`time:<value>:<record>`); completed steps are not repeated on retry or when resuming a wait, and resuming uses `FOR UPDATE SKIP LOCKED`.
- **Isolation**: queries and jobs always with the event's organization; the run uses the publisher's scope in that organization.
- **Cycles**: maximum depth of 5, detection of the same automation in the causation chain and a limit of 20 runs per automation and record per hour.
- **Concurrency**: named lock per (automation, record).
- **Permissions**: the automation never does more than its publisher can do, checked on publish and on run.
- **Secret-free history**: secrets stay in the vault; inputs mask sensitive fields; internal errors show up as `internal error` (details only in the log).
- **Traceability**: events and audit with the `automation` actor and `causation`.

## Configuration packages

The `automations` section (`modules/automations/section.rs`) exports each automation's draft, replacing ids with portable keys; installation does the reverse, using first the items installed by the package itself (`ApplyContext.resolved`) and then the organization's:

| In the organization | In the package |
| --- | --- |
| `config.tag_id` (`add_tag`/`remove_tag`) | `config.tag`: tag key |
| `config.stage_id` (`move_stage`) | `config.stage`: `<pipeline>.<stage>` (resolved by the `PIPELINE_STAGE_ITEM` key `<pipeline>/<stage>` recorded by the `pipelines` section in the same installation, or by the stage keys already recorded in the organization) |
| `config.activity_type_id` (`create_task`) | `config.activity_type`: activity type key |
| `config.sequence_id` (`enroll_in_sequence`) | `config.sequence`: sequence key (from the package's `sequences` section or from the organization) |
| `trigger.from_stage_id`/`to_stage_id` (`deal.stage_changed`) | `trigger.from_stage`/`to_stage`: `<pipeline>.<stage>` |
| `trigger.segment_id` (`time.schedule`) | `trigger.segment`: `<record_type>/<segment name>` |
| `value` of `in_segment` conditions (in any filter of the trigger or of the definition) | `"<record_type>/<segment name>"` instead of the uuid |

Segments are not part of packages: `segment` and `in_segment` conditions by key must point to a segment that already exists in the organization (otherwise validation reports `unknown_reference`); a uuid in `in_segment` is left as is. Every reference by key is checked during validation (`unknown_reference`); if it still does not resolve when applying, installation fails with a validation error listing the items that were not installed.

## Known limitations

- Time-based triggers only look forward from publication (records already due at publication do not fire), except `time.deal_rotting`, which catches up on deals that are already rotting.
- Continuing a full batch uses an offset (`page_offset`) over the field's ordering: if records change in the middle of a large batch, one may be left out of that window (it never fires twice).
- The `time.deal_rotting` catch-up does not happen when only the stage rule changes in some other way (for example, a reactivated stage) nor when a reopened deal is already rotting: those cases depend on `rotting_at` falling inside the window.
- `no_activity` and `same_stage` depend on `last_activity_at`/`stage_entered_at`, which currently only deals have.
- The development clock offset does not affect relative filters or the queue's `run_at`.
- `record.updated` on deals fires one run per event: each event carries only its own fields (with no repetition between `deal.updated`, `deal.stage_changed`, `deal.pipeline_changed`, `deal.amount_changed`, `deal.owner_changed` and `deal.expected_close_date_changed`), so an automation filtered by `fields` fires once per change, but one without `fields` runs once per event when the same edit changes fields from different events (for example title and amount).
- Updating activity fields through automations is not supported (only creating tasks and notes from them).
- On custom objects, `create_task` creates the task without a link to the record (activities cannot yet be linked to `custom.*`, pending from session 16).
- Fixed users, `incoming_webhook_id` and ids inside filters are not translated to keys in configuration packages.
