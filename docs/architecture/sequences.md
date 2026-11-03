# Sequences (cadences)

A sequence is an ordered list of steps that follows a contact: automatic emails, tasks and waits in business days. The contact leaves on its own when there is a sign of engagement or of disinterest. Module `sequences` (depends on `email`, `people` and `tasks`): code in `back/modules/sequences` (`service.rs` with the use cases over `Ctx`, `store.rs` with the SQL, `runner.rs` with step execution, `exits.rs` with the exits, `runtime.rs` with jobs, consumer and worker, `http.rs`/`dto.rs`, `hooks.rs`), package section in `modules/config_packages/src/application/sections/sequences.rs`, module migration `0001_create_sequences.sql` (formerly `2310`–`2313`), web in `web/src/modules/sequences`. Each step's email goes out through the `email::Sender` service. Before running an email or task step, the `sequences::StepRunning` hook (veto, `RunningStep`) is dispatched: a permanent error takes the contact out of the sequence (`removed`), a transient error makes the job try again.

## Model

| Table | What it stores |
| --- | --- |
| `sequences` | Stable key, name, description, `is_active`, sending window (`window_start_minute`/`window_end_minute`, minutes of the day in the organization's time zone) and `daily_limit` (sequence emails per mailbox per day) |
| `sequence_steps` | Steps in order (`position`): `auto_email`, `manual_email`, `call`, `linkedin`, `task` or `wait` (`wait_days` in business days); emails with `template_id` (organization template) and/or `subject` + `body_html`; tasks with `title` |
| `sequence_enrollments` | A contact's enrollment: who enrolled it (`enrolled_by`), sending mailbox, origin (`manual`, `bulk`, `automation`), `status` (`active`, `paused`, `completed`, `exited`), `current_position` (next step), `next_run_at`, last email sent (to reply in the same conversation), exit reason and step. The generated column `live_contact_id` + a unique index prevent two live enrollments of the same contact in the same sequence |
| `sequence_step_runs` | Execution of a step per enrollment (unique per `(enrollment_id, position)`): status (`running`, `sent`, `created`, `failed`), `email_id`/`thread_id` or `activity_id`, and the `opened_at`, `replied_at`, `bounced_at` marks used in the metrics |

## Enrollment

`sequences::service::enroll` requires `sequences.enroll` and read access to the contact (`RecordAccessChecker`), an active sequence with at least one step that is not a wait and, if there is an `auto_email`, a contact with an email, no objection and a mailbox of the enroller that is not disconnected. Three entry points use the same use case: `POST /sequences/{id}/enrollments` (the contact's **Sequences** tab), the bulk action `enroll_in_sequence` (`BulkAction::EnrollInSequence`, people only, from the selection or the view's filter) and the automation action `enroll_in_sequence` (`AutomationGateway::enroll_in_sequence`; enrolls as the contact's owner, or as whoever published the automation with `sender: automation_author`; a repeated enrollment, a contact without email or with an objection end the step with `enrolled: false`). Pausing, resuming and removing (`/sequence-enrollments/{id}/pause|resume|exit`) require `sequences.enroll` with scope over the enroller; reading sequences, enrollments and metrics uses `sequences.read` (own/team/organization over `enrolled_by`) or `sequences.manage`.

## Execution

- `sequences.tick` (scheduled every minute) lists the `active` enrollments with an overdue `next_run_at` in active sequences (query without user scope, like `MailboxStore::due_for_sync`) and enqueues one `sequence.step` per enrollment with `unique_key = sequence.step:<id>`.
- `sequence.step` (`sequences::runner::process`) rereads the enrollment and runs the current step with the enroller's current scope (`member_scope`; if the enroller left the organization the enrollment ends with `enroller_unavailable`).
  - `wait`: advances and sets `next_run_at` to N business days later, in the organization's time zone (`organization_settings.timezone`).
  - `auto_email`: only within the window on a business day; at most one sequence email per contact per day and `daily_limit` per mailbox per day. Outside that, the step is **postponed** to the next valid opening (never skipped). Sending uses the `email::Sender` service with the enroller's scope (the enrollment's mailbox, record = contact, templates/variables/tracking from the email module); from the second email on it replies to the last one sent (same conversation, `Re:`).
  - `manual_email`, `call`, `linkedin`, `task`: create an activity through the tasks use case (types `email`, `call` and `task`), assigned to the enroller, linked to the contact and due immediately; the manual email carries the rendered draft (subject and text) in the description. The sequence advances without waiting for the task to be completed.
- Idempotency: before acting, the step is claimed in `sequence_step_runs` (`claim_run`); a repeated job that finds the run `sent`/`created` only advances, and a run left `running` (crash midway) is marked `failed` and the sequence moves on, without resending. A transient provider error releases the claim and the job tries again; a permanent error or the last attempt ends the enrollment with `send_failed`.
- After the last step the enrollment becomes `completed` (`sequence.completed`).

## Automatic exits

The `sequences.exits` consumer (`sequences::exits::handle`) ends live enrollments with `sequence.exited` (`Actor::System`) and the reason:

| Event | Reason |
| --- | --- |
| Inbound `email.received` of type `message` | `replied`: the enrollment of the event's contact exits, as does any enrollment whose sent email is in the same conversation (which also gets `replied_at`) |
| `email.bounced` | `bounced` (enrollment of the bounced email and of the contact) |
| `meeting.booked` (`MeetingEventPayload.contact_id`) | `meeting_booked` |
| `deal.won` / `deal.lost` | `deal_won` / `deal_lost` for the deal's primary contact and additional people |
| `contact.deleted` | `contact_deleted` |
| `email_tracking.objection_added` | `objection` |

`email.opened` only sets the run's `opened_at`. **Objection**: the core has no general registry of objections to processing; the signal used is `email_tracking_objections` (art. 18, § 2 of the LGPD, managed with `privacy.manage`), comparing the `organization:address` hash with the email of the enrolled people. The same check runs before each automatic email and on enrollment. Anonymization by the privacy module erases the contact's email, and the next email ends the enrollment with `contact_unavailable`.

## Events and audit

`sequence.created|updated|deleted`, `sequence.enrolled`, `sequence_enrollment.paused|resumed`, `sequence.step_executed`, `sequence.completed` and `sequence.exited`, always with audit in the same transaction. Enrollment events have the contact as their record (they appear on the timeline and feed automations and webhooks); `sequence.enrolled|completed|exited` are in the public catalog with the contact in `data`.

## Configuration packages

Section `sequences` (`SequencesSection`), reconciled by key; a step's email template goes by `template_key`. See [`docs/config-packages/README.md`](../config-packages/README.md#sequences).
