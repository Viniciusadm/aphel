# Calls

A call is a (completed) **activity of type `call`** with its own details. The core does not talk to any carrier: dialing is `tel:` in the browser or a telephony provider contributed by a module.

## Model

| Table | What it stores |
| --- | --- |
| `activity_calls` | 1:1 with `activities`: direction (`inbound`/`outbound`), `outcome_key`, `duration_seconds`, phone, `recording_url`, `provider` + `provider_call_id` (unique) and `session_id` |
| `call_outcomes` | Configurable outcomes: stable key, name, `is_connected` (counts as answered in the report), position, active. The system ones (`answered`, `voicemail`, `no_answer`, `wrong_number`) are seeded at installation (`seed_call_outcomes`, called by `seed_defaults`) |
| `call_sessions` | A user's call session: source view, script, state (`active`/`completed`), current position and total |
| `call_session_items` | The session's frozen queue: contact by position, state (`pending`, `logged`, `skipped`), last call and outcome |

`ActivityView.call` (and `ActivityResponse.call`) comes from the `LEFT JOIN` in `activity_select`; the timeline receives `metadata.call` (direction, outcome, duration, whether it has a recording), which the web app shows in the `activity` renderer registered in `web/src/features/calls/timeline`.

## Use cases (`modules/calls`)

`CallUseCases<P: CallPorts, A: ActivityPorts>` reuses `ActivityUseCases`: `prepare_create` validates the activity and `insert_prepared` writes it inside the call's transaction. Logging = `begin` → activity (`activity.created`) → `activity_calls` → `call.logged` + audit → `commit`.

- `log` (`POST /calls`), `update` (`PATCH /calls/{id}`, `If-Match` on `call.updated_at`, `call.updated` event with `changes`).
- `record_event` (`POST /calls/events`): provider event; the first `provider_call_id` logs the call, the following ones update it. Without `outcome_key`, it uses the first active connected outcome (duration > 0) or not connected outcome.
- `start` (`POST /calls/start`) and `recording` (`GET /calls/{id}/recording`) delegate to the provider.
- Sessions: `start_session` runs the `RecordQuery` of the people saved view (the view's filter and sort, the user's read scope, up to 500) and freezes the queue. `log_in_session` logs the call for the contact at the current position and, in the same transaction (session locked with `FOR UPDATE`), marks the item and advances to the next pending one (after the position, then from the start); with no pending items, it ends the session (`call_session.completed`). `skip_in_session`, `move_session` (back/skip) and `finish_session`. Skipping and moving are navigation state: they do not generate events.
- `report` (`GET /calls/report`): count and duration by owner and outcome, under `scope.record_filter(activities.read)`.
- Outcomes: `list_outcomes`, `create_outcome`, `update_outcome`, `reorder_outcomes` (`settings.manage`; `call_outcome.*` events). The configuration lives in the **Outcomes** tab of `/calls`.

Permissions: `activities.create` logs calls and runs sessions, `activities.update` edits a call, `activities.read` reads, lists sessions and sees the report. Sessions always belong to the user themselves.

## Telephony via modules

VoIP providers implement `calls::telephony::TelephonyProvider` and come in as a typed contribution (`app.contribute::<dyn TelephonyProvider>(..)`); the `calls` module's `dyn TelephonyRouter` service picks the provider (only enabled modules). To report the call (answered, ended, recording ready), the provider calls `POST /v1/calls/events`, which logs or updates the call by `provider_call_id`.

On the web, the records' phone (`PhoneValue` in `features/records/display`) uses the renderer registered by `features/calls`: it opens `tel:` (or starts through the first available provider) and opens the call logging dialog already linked to the record.
