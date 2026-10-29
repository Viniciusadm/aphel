# Lead scoring

Scoring models that add up points from the record's **profile** (`RecordQuery` filters) and **behavior** (signals coming from domain events), with decay over time, cold/warm/hot bands, an automation trigger when a band is crossed and an explanation of the score on the record detail.

Code: `scoring` module (`back/modules/scoring`, session 31; depends on `contacts` and `companies`, enabled by default): `domain.rs` (model, signals, calculation), `engine.rs` (`ScoringEngine` over `&Ctx`), `service.rs` (use cases as functions over `&Ctx`), `runtime.rs` (worker consumer and jobs, `ScoringRuntime`), `store.rs` (`ModelStore`, `ScoreStore`), `directory.rs` (queries through the record engine), `automation.rs` (trigger), `section.rs` (package), `http.rs` (API `/api/v1/scoring`) and `migrations/0001_create_scoring.sql`. Web: `web/src/features/settings/scoring` (route `/settings/scoring` and the "Score" tab on the contact and company detail).

`mod.rs` registers everything by itself: the `score` field on `contact` and `company` (`app.extend_record_type(.., |t| t.computed(score_field(..)))`), the `scoring.events` consumer, the `scoring.recalculate`/`scoring.decay` jobs and the schedule (`app.worker`), the routes, the OpenAPI tag, the `scoring.manage` permission (granted to `admin` via `grant_to`) and the public events `score.band_changed` (contact) and `score.company_band_changed` (company). With the module disabled, the consumer, jobs and routes stop (the `score` field stays in the record type registry).

## Model

Tables `scoring_models` + `scoring_criteria`.

| Field | Meaning |
| --- | --- |
| `key` | Stable, unique key (snake_case), used by configuration packages and by the trigger. Generated from the name. Never changes. |
| `record_type` | `contact`, `company`, `deal` or `co_<key>` (`is_scored_record_type`). Never changes. |
| `is_primary` | One primary model per type (unique by `primary_record_type`, a generated column). The type's first model becomes primary; marking another one unmarks the previous one. |
| `is_active` | An inactive model does not compute scores and does not show up on the record detail. |
| `warm_from` / `hot_from` | Bands: `score < warm_from` = `cold`, `< hot_from` = `warm`, otherwise `hot`. Default 30/70; `hot_from > warm_from`. |
| `criteria` | Up to 50 criteria, each with a stable `key` within the model (the criterion id is preserved by the key when the model is edited). |

Criteria:

- **Profile** (`kind = profile`): a `filter` in the views format (`Filter` AST), validated against the type's fields with `validate_view_query_related` (errors in `criteria.<n>.filter...`), and `points` (−1000 to 1000, nonzero; negative subtracts points). The record earns the points while it matches the filter. The filter cannot use the `score` field itself. The filter is evaluated by the `RecordQueryEngine` with system scope (all fields, whole organization, archived records included), never SQL built from user text.
- **Behavior** (`kind = behavior`): `signal`, `points` per occurrence, optional `max_points` (limit on the criterion's sum, in absolute value) and optional `decay_days`.

### Formula

For each behavior criterion, each occurrence is worth

```
points × max(0, 1 − age_in_days / decay_days)
```

(linear down to zero at `decay_days`; without `decay_days` the occurrence is worth its points forever). The sum of the occurrences is capped by `max_points` (minimum of `max_points` for positive points, maximum of `−max_points` for negative ones) and rounded to an integer. The score is the sum of the matched profile criteria and the behavior criteria. The explanation (`record_scores.explanation`) stores, for each criterion that contributed, `{ criterion_id, key, label, kind, points, occurrences }`.

## Behavior signals

`application/scoring/signals.rs` translates domain events into signals (`BehaviorSignal`). Each signal's idempotency key is `(type, record, signal, source_key)` (UNIQUE in `scoring_signals`), so reprocessing the same event ("at least once" delivery) does not duplicate it.

| Signal | Event | `source_key` | Records |
| --- | --- | --- | --- |
| `email_opened` | `email.opened` (only the first open of the message) | `email_id` | the event's record (one event per linked record) |
| `email_clicked` | `email.clicked` | `email_id` | same |
| `email_replied` | `email.received` with `direction = inbound` and `kind = message` | `email_id` | same |
| `meeting_booked` | `meeting.booked` | `booking_id` | contact |
| `form_submitted` | `lead_form.submitted` (`lead_forms` module) | `submission_id` | contact |
| `call_connected` | `call.logged`/`call.updated` with `is_connected = true` | `activity_id` (the call counts once even if the outcome is edited) | the call's record (`payload.records`) |

Signals are stored per record (independent of models), so a new criterion or model starts counting the history within the decay window. Signals are only recorded when the organization has at least one model (active or not).

**Company:** contacts' signals count toward company models through the contact's current company (`contacts.company_id`, non-deleted contacts), in addition to the signals linked directly to the company.

## Recalculation

Everything goes through `ScoringEngine` and is idempotent: it computes the score from the state (filters + signals + clock), compares it with the stored one (`SELECT … FOR UPDATE`) and only writes what changed. A record with no points and no explanation has no row (score 0, cold band).

- **`scoring.events` consumer**: signals (table above) and `<type>.created|updated|restored|deleted|purged` of `contact`, `company`, `deal` and `co_*`. Recalculates the affected records in the type's active models; every contact event recalculates the contact's current company, and a `company_id` change also recalculates the old one; `<type>.deleted` deletes the record's scores and `<type>.purged` (permanent deletion from the trash) also deletes the signals.
- **`scoring.recalculate` job** (`{ model_id }`): full recalculation of the model. Enqueued when the model is created, when criteria/bands/activation change, by `POST /scoring/models/{id}/recalculate` and by package installation.
- **Daily `scoring.decay` job** (`0 30 3 * * *`): full recalculation of all active models. Applies decay and corrects drift from relative filters (dates, segments, relationships).

The full recalculation considers the union of: records matching some profile filter, records with signals within the window of the longest decay, and records that already have a score.

## Bands and automation

When a record's band changes, the engine publishes `score.band_changed` (`score.company_band_changed` for companies; `system` actor, record = the scored record) with `{ model, model_key, record_type, record_id, old_band, new_band, band, direction (up|down), old_score, score, changes }` and writes an audit entry, in the same transaction as the score.

Automation trigger `score.band_changed` (`modules/scoring/automation.rs`, in `TriggerRegistry::builtin()`, listens to both events): types `contact`, `company`, `deal`, `co_*`; options `model_key`, `band` (`cold|warm|hot`, the new band) and `direction` (`up|down`).

The event is in the public catalog with the Contact resource (webhooks deliver only contact events; the catalog has one resource per event).

## `score` system field

`score` (number, read-only, filterable and sortable) on contact and company, contributed by `scoring` (a computed contributed field, see [`record-pipeline.md`](record-pipeline.md#contributed-fields)): `SystemField::computed(ComputedColumn::PrimaryScore { record_type })`, SQL in `kernel/infrastructure/query/computed.rs` — the score of the type's **primary and active** model (0 without a score; empty without an active primary model). The scores of all models are available at `GET /api/v1/scoring/records/{record_type}/{id}`.

## API

`scoring.manage` (administrative permission, granted to the `admin` role via `grant_to`) on `/scoring/models`:

- `GET /scoring/models?record_type=` — list with criteria;
- `POST /scoring/models` — creates (`scoring_model.created`) and enqueues the recalculation;
- `GET|PATCH|DELETE /scoring/models/{id}` — reads, updates (`If-Match`, `scoring_model.updated`; `criteria` replaces the list) and removes (`scoring_model.deleted`, the scores go with it);
- `POST /scoring/models/{id}/recalculate` — 202, enqueues the recalculation.

`GET /scoring/records/{record_type}/{id}` requires read access to the record (same visibility filter as the list) and returns the score, band and explanation in each active model of the type, primary first.

Every model mutation writes an event and an audit entry in the same transaction. `scoring_model.*` is internal.

## Configuration package

Section `scoring_models` (`scoring::section::ScoringModelsSection`): `{ key, record_type, name, description?, primary, active, warm_from, hot_from, criteria[] }`, criteria with `{ key, label, kind, filter?, signal?, points, max_points?, decay_days? }`. Reconciled by the model key; criteria by the key within the model. Installing creates or updates and never deletes (criteria missing from the package are kept); a different `record_type` is an unresolvable conflict. Filters are only checked for shape on installation (the fields may come in the same package); a filter that stops resolving is ignored in the calculation with a warning in the log. Installation enqueues the recalculation of the written models.

## Web

- `/settings/scoring` (`ScoringPage`): list of models with activate/deactivate, make primary, recalculate, edit and delete. The editor (`ScoringModelDialog`) uses the `FilterBuilder` from `features/records/filters` for profile criteria.
- "Score" tab on the contact and company detail (`useScoringTab`/`RecordScorePanel`): score, band and explanation per model.
- i18n namespace `scoring`.

## Pending items

- The SQL expression for `score` (`ComputedColumn::PrimaryScore`) still lives in the kernel (`kernel/infrastructure/query/computed.rs`); computed columns are not an open catalog.

- Signals of permanently deleted records stay in `scoring_signals` (no cleanup on purge).
- Signals prior to the organization's first model are not recorded.
- `score` only exists as a system field on contact and company; for deals and custom objects the score is only available in the API and in the explanation.
- The consumer does not recalculate companies when a company profile filter depends on contact data (`contacts` relationship); the daily job corrects it.
