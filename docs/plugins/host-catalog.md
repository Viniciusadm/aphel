# Host catalog

What this Aphel offers to runtime plugins besides records: the hooks a manifest can subscribe to in `[[hooks.subscribe]]` and the operations its backend can call with `invoke` after listing them in `operations`. The live list, including whether each module is enabled, is `GET /api/v1/wasm-runtime/host-catalog`. How they work: [README](README.md#module-operations-and-domain-hooks).

## Hooks

| Hook | Module | Record type | Kind | Values |
| --- | --- | --- | --- | --- |
| `record.archiving` | wasm_runtime | any | veto | The custom record or company being archived |
| `record.committed` | wasm_runtime | any | action | `values` and `changes`, after the commit (errors only logged) |
| `record.deleted` | wasm_runtime | any | action | The deleted record, inside the transaction |
| `record.deleting` | wasm_runtime | any | veto | The record being deleted |
| `record.purged` | wasm_runtime | any | action | The purged record, inside the transaction |
| `record.restored` | wasm_runtime | any | action | The restored record, inside the transaction |
| `record.saved` | wasm_runtime | any | action | `values` and `changes`, inside the transaction |
| `record.saving` | wasm_runtime | any | filter | The draft `values`/`custom_fields`; return changes |
| `record.validating` | wasm_runtime | any | veto | `values` and `changes` of the write |
| `activity.completing` | tasks | `tasks.task` | veto | Before an activity is completed; values: { activity, previous, type_key }. An error blocks it |
| `automation.action_running` | automations | any | veto | Before an automation action runs on a record (dry run and execution), for the record's type; values: { automation_id, run_id, step_id, action, record: { record_type, record_id }, config, dry_run }. An error skips the action |
| `deal.losing` | deals | `deals.deal` | veto | Before a deal is lost; values: { deal, previous, stage, lost_reason }. An error blocks it |
| `deal.lost` | deals | `deals.deal` | action | After a deal is lost, inside the transaction; values: { deal, previous, stage, lost_reason } |
| `deal.stage_changing` | deals | `deals.deal` | veto | Before a deal moves to another stage; values: { deal, previous, from, to }. An error blocks the move |
| `deal.winning` | deals | `deals.deal` | veto | Before a deal is won; values: { deal, previous, stage, lost_reason }. An error blocks it |
| `deal.won` | deals | `deals.deal` | action | After a deal is won, inside the transaction; values: { deal, previous, stage } |
| `email.outgoing` | email | `email` | filter | Before an e-mail is saved and sent; record_type is the linked record's type (or email when unlinked); values: { mailbox_id, sender_id, from, to, cc, bcc, subject, body: { text, html }, record, thread_id, scheduled, tracking }. Return { subject?, body?: { text?, html? } } to change them (html is sanitized) |
| `lead.converted` | leads | `people.person` | action | After a lead was converted, inside the transaction; values: { lead, deal_id, company_id } |
| `lead.converting` | leads | `people.person` | filter | Values of the deal (and optional company) a lead conversion creates; values: { contact_id, lead, deal, company }. Return { deal?: {..}, company?: {..} } to merge field values into them (a company object makes the conversion create a company when the contact has none) |
| `lead.disqualifying` | leads | `people.person` | veto | Before a lead is disqualified; values: { lead, reason: { id, key, name }, note }. An error blocks it |
| `lead.qualifying` | leads | `people.person` | veto | Before a lead is qualified in the lead inbox; values: { lead }. An error blocks the qualification |
| `quote.accepting` | quotes | `quotes.quote` | veto | Before a quote is accepted (public page or manually); values: { quote, deal_id, acceptor: { name, email }, manual }. An error blocks the acceptance |
| `sequence.step_running` | sequences | `people.person` | veto | Before a sequence step runs for an enrolled contact; values: { enrollment_id, sequence_id, contact_id, enrolled_by, position, kind, template_id }. A definitive error removes the contact from the sequence, a transient one retries the step |

## Operations

Writing operations are refused inside hooks (409 `plugin_write_in_hook`). Each one needs the same permissions as the module's API, so list them in `requires`. Event, schedule, HTTP response and automation callbacks are retryable: they may run again after a failure. There, a writing operation runs synchronously only when it is **idempotent** (repeating it with the same input changes nothing; runtime plugins declare it with `idempotent = true` in `[[provides.operations]]`); any other writing operation must go through `defer-effect` with an `invoke` command, which only accepts operations marked **durable**: they write everything in one transaction, without external calls, so a retry never repeats them ([durable execution](durable-execution.md)). External queries (`ai.*` and `enrichment.lookup`) use synchronous `invoke` with persisted result reuse in retryable callbacks; idempotent external operations are simply called again. The live catalog exposes `external_query`; they remain blocked in hooks and read-only widgets. Delivery operations commit queue acceptance; provider completion happens in the worker. See [delivery and query guarantees](durable-execution.md#queued-module-delivery).

| Operation | Module | Access | Durable | Input and result |
| --- | --- | --- | --- | --- |
| `ai.complete` | ai | write | no | Free completion with the configured AI provider, counted in the daily and per-user limits and logged as feature plugin: { system?, prompt } or { system?, messages: [{ role: user\|assistant, content }] }, max_tokens? (1-4096, default 1024), temperature? (0-1) -> { text, input_tokens, output_tokens, provider, model } |
| `ai.compose` | ai | write | no | Write or rewrite a text: { context: note\|email\|message, instruction, draft?, record_type?, record_id? } -> { text, sources, provider, model } |
| `ai.next_actions` | ai | write | no | Up to 5 suggested next actions for a record: { record_type: contact\|company\|deal, record_id } -> { suggestions: [{ title, due_in_days, reason }], sources, provider, model } |
| `ai.summary` | ai | write | no | AI summary of a record the actor can read: { record_type: contact\|company\|deal, record_id } -> { text, sources, provider, model } |
| `approvals.approve` | approvals | write | yes | Approve a request as an approver of its current step (needs a user actor): { id, comment? } |
| `approvals.cancel` | approvals | write | yes | Cancel a pending request (needs a user actor): { id, comment? } |
| `approvals.reject` | approvals | write | yes | Reject a request as an approver of its current step (needs a user actor): { id, comment } |
| `approvals.requests` | approvals | read | — | Approval requests of a record the actor can read: { record_type, record_id, status?, page?, per_page? } -> { data, total } |
| `approvals.revoke` | approvals | write | yes | Revoke an approval that was not used yet (needs approvals.manage and a user actor): { id, comment? } |
| `approvals.submit` | approvals | write | yes | Ask for the approvals a record needs at a moment (needs edit access): { record_type, record_id, moment: on_demand, stage_change, quote_send or a contributed moment, target_id?, comment?, policy_key? } -> { outcome, requests } |
| `assignment.assign` | assignment | write | yes | Pick the next owner of a record with a lead distribution rule and set its owner_id (needs update access to the record): { record_type, record_id, rule? (rule id; optional when exactly one active rule distributes the type), idempotency_key? } -> { rule_id, user_id, user_name, selection, route_index, reused, skipped } |
| `booking.bookings` | booking | read | — | Meeting bookings of the hosts within the actor's booking.manage scope: { status?: confirmed\|canceled, meeting_type_id?, upcoming?, page?, per_page? } -> { data, total } |
| `booking.cancel` | booking | write | yes | Cancel a booking as the host (the guest receives the cancellation): { id, reason? } |
| `currencies.convert` | money | read | — | Today's exchange rate of a currency to the base currency: { currency } -> { base, currency, enabled, rate }; or convert an amount: { amount, from, to? (default: base currency) } -> { amount, from, to, base, rate, converted } (decimals as strings, rate/converted null when a rate is missing) |
| `deals.move_pipeline` | deals | write | yes | Move a deal to another pipeline: { id, pipeline_id, stage_id?, lost_reason_id?, lost_reason_note?, fields? } |
| `deals.move_stage` | deals | write | yes | Move a deal to a stage (a won/lost stage wins/loses it): { id, stage_id, lost_reason_id?, lost_reason_note?, fields? } |
| `duplicates.check` | duplicates | read | — | Visible records the enabled rules consider possible duplicates of the given values (at most 10); nothing is stored: { record_type, exclude_id?, name?, first_name?, last_name?, email?, phone?, mobile?, company_id?, domain?, website?, values?: { field_key: text } } -> [{ record_id, title, rules }] |
| `duplicates.dismiss` | duplicates | write | yes | Mark a duplicate candidate as not a duplicate: { id } -> candidate |
| `duplicates.merge` | duplicates | write | yes | Merge duplicate_id into primary_id (the duplicate goes to the trash): { record_type, primary_id, duplicate_id, fields?: { field_key: primary\|duplicate }, expected_updated_at? (primary's updated_at, ISO-8601) } -> { record_type, survivor_id, merged_id, moved } |
| `email.link` | email | write | yes | Link an e-mail conversation to a record by hand: { thread_id, record_type, record_id } |
| `email.send` | email | write | yes | Send (or schedule) an e-mail from one of the acting user's mailboxes through the delivery queue: { to: [{ address, name? }], cc?, bcc?, mailbox_id?, subject?, body_html?, body_text?, template_id?, record?: { record_type, record_id }, reply_to_message_id?, attachment_file_ids?, scheduled_at?, tracking? } |
| `email.unlink` | email | write | yes | Remove the link between an e-mail conversation and a record: { thread_id, record_type, record_id } |
| `enrichment.apply` | enrichment | write | yes | Apply chosen items of a lookup (within 60 minutes) to the company: { lookup_id, items: [data_key], only_blank? } -> { company_id, source, cnpj, fields } |
| `enrichment.lookup` | enrichment | write | no | Look up a company's CNPJ in the configured public source and cache the field-by-field proposal without changing the company: { company_id, cnpj? } -> { id, company_id, cnpj, source, fetched_at, items: [{ data_key, label, field_key, current, proposed, changed }] } |
| `esign.cancel` | esign | write | yes | Queue cancellation of an open signature request (needs esign.send): { id } |
| `esign.send` | esign | write | yes | Queue a PDF attached to a deal for electronic signature (needs esign.send): { deal_id, file_id, signers: [{ name, email, contact_id? }], message?, quote_id? } |
| `exports.request` | exports | write | yes | Queue a CSV or XLSX export of records the actor can read (at most 100 000); the requester is notified when the file is ready: { record_type, format: csv\|xlsx, filter?, sort?, search?, archived?: exclude\|include\|only, columns? } -> { id, record_type, format, status, columns, file_id, row_count, failure_reason, requested_by, created_at, .. } |
| `favorites.add` | favorites | write | yes | Add a favorite for the acting user (needs a user actor): { favorite_type: record\|view, record_type?, record_id?, view_id? } |
| `favorites.list` | favorites | read | — | Favorites of the acting user in display order (needs a user actor): {} -> { data } |
| `favorites.remove` | favorites | write | yes | Remove a favorite of the acting user (needs a user actor): { id } |
| `followers.follow` | followers | write | yes | Make the acting user follow a record (needs a user actor): { record_type, record_id } |
| `followers.list` | followers | read | — | Followers of a record the actor can read: { record_type, record_id } -> { record_type, record_id, following, total, followers } |
| `followers.unfollow` | followers | write | yes | Make the acting user stop following a record (needs a user actor): { record_type, record_id } |
| `forecast.deals` | forecast | read | — | Open deals expected to close in a month, largest first: { month: YYYY-MM, category?: pipeline\|best_case\|commit, owner_id?, team_id?, page?, per_page? } -> { data, total } |
| `forecast.summary` | forecast | read | — | Revenue forecast per month from the current month, within the actor's deals.read scope: { owner_id?, team_id?, months? (at most 18) } -> { currency, timezone, from, to, probability_source, settings, months: [..], totals } |
| `goals.get` | goals | read | — | One goal with its progress: { id } |
| `goals.list` | goals | read | — | Goals the actor reaches, newest period first, each with its progress: { owner_type?: user\|team\|organization, user_id?, team_id?, metric?, period?, active_on? (YYYY-MM-DD), current?, progress? (default true), page?, per_page? } -> { data, total } |
| `leads.convert` | leads | write | yes | Convert an open lead into a deal: { contact_id, pipeline_id, stage_id?, title?, amount?, currency? } -> { contact_id, lifecycle_stage, deal_id } |
| `leads.disqualify` | leads | write | yes | Disqualify an open lead with a reason: { contact_id, reason_id, note? } -> { contact_id, lifecycle_stage, deal_id } |
| `leads.qualify` | leads | write | yes | Qualify a contact in the lead stage: { contact_id } -> { contact_id, lifecycle_stage, deal_id } |
| `line_items.list` | line_items | read | — | Line items and totals of a record: { record_type, record_id } |
| `line_items.replace` | line_items | write | yes | Replace all line items of a record: { record_type, record_id, items: [{ product_id?, name?, unit?, quantity?, unit_price?, discount_type?: none\|percent\|amount, discount_value?, tax_percent?, recurrence?: none\|monthly\|quarterly\|yearly, term_months? }] } |
| `messaging.reply` | messaging | write | yes | Queue a reply in a shared inbox conversation (needs messaging view and reply permissions): { conversation_id, body } |
| `notes.create` | notes | write | yes | Create a note on a record: { record_type, record_id, body, visibility?: team\|private, pinned? } |
| `notes.delete` | notes | write | yes | Delete a note: { id } |
| `notes.get` | notes | read | — | One note: { id } |
| `notes.list` | notes | read | — | Notes of a record visible to the actor: { record_type, record_id, page?, per_page? } -> { data, total } |
| `notes.pin` | notes | write | yes | Pin or unpin a note: { id, pinned } |
| `notes.update` | notes | write | yes | Edit a note: { id, body?, visibility?, pinned? } |
| `pricing.list` | pricing | read | — | Price lists, the default of each currency first: { currency?, active?, search?, page?, per_page? } -> { data, total } |
| `pricing.resolve` | pricing | read | — | Unit price of a product from a price list (quantity tiers applied): { product_id, price_list_id? , currency? (required without price_list_id; picks the list for the company or the currency default), company_id?, quantity? (default 1), on? (YYYY-MM-DD, default today) } -> { price_list: { id, name, currency } \| null, unit_price (string) \| null, currency, quantity, on } |
| `proposals.generate` | proposals | write | yes | Generate a PDF proposal for a deal and attach it (needs proposals.generate and edit access to the deal; the default active template when none is given): { deal_id, template_id?, template_key?, quote_id? } |
| `publication.cancel` | publication | write | yes | Cancel an open publication request: { id } -> request |
| `publication.capture` | publication | write | yes | Capture the saved draft of a publishable record as a revision: { record_type, record_id, expected_version, note? } -> revision |
| `publication.request` | publication | write | yes | Publish a revision now or at a time (UTC), subject to approval policies: { revision_id, at?, idempotency_key } -> request |
| `publication.restore` | publication | write | yes | Copy a revision back into the draft fields: { revision_id, expected_version } -> record |
| `publication.retry` | publication | write | yes | Retry a failed publication request: { id } -> request |
| `publication.revision` | publication | read | — | One revision with its payload: { id } -> revision |
| `publication.revisions` | publication | read | — | Revisions of a record without payloads: { record_type, record_id, page?, per_page? } -> { data, total } |
| `publication.status` | publication | read | — | Publication status of a record: { record_type, record_id } -> status |
| `publication.withdraw` | publication | write | yes | Withdraw the published revision of a record: { record_type, record_id, idempotency_key } -> request |
| `quotes.accept` | quotes | write | yes | Record the acceptance of a quote on behalf of the customer: { id, name?, email?, note? } |
| `quotes.new_version` | quotes | write | yes | Create a new version of the current quote: { id } |
| `quotes.send` | quotes | write | yes | Queue the quote link e-mail to the customer: { id, to?: [{ address, name? }], cc?: [{ address, name? }], subject?, body_html?, mailbox_id? } |
| `quotes.submit` | quotes | write | yes | Send a draft quote for approval (approved right away when no policy applies): { id, comment? } |
| `records.archive` | wasm_runtime | write | yes | Archive a record (hides it from default lists; not a deletion): { record_type, id }. Supported by contact, company, deal and custom object (custom.*) records; other types fail with 422 record_operation_not_supported |
| `records.restore` | wasm_runtime | write | yes | Unarchive an archived record (not a trash restore): { record_type, id }. Supported by contact, company, deal and custom object (custom.*) records; other types fail with 422 record_operation_not_supported |
| `reminders.create` | reminders | write | yes | Create a reminder for the acting user (needs a user actor): { remind_at, note?, record_type?, record_id? } |
| `reminders.delete` | reminders | write | yes | Delete a reminder of the acting user (needs a user actor): { id } |
| `reminders.list` | reminders | read | — | Reminders of the acting user sorted by remind_at (needs a user actor): { status?: pending\|sent\|dismissed, record_type?, record_id?, page?, per_page? } -> { data, total } |
| `reports.list` | reports | read | — | Saved reports visible to the actor, ordered by name: { record_type?, page?, per_page? } -> { data, total } |
| `reports.run` | reports | read | — | Run a saved report with the actor's read scope and field permissions: { id, preset?, from? (YYYY-MM-DD), to?, compare_to_previous? } -> { record_type, timezone, groups, metrics, rows, totals, period, previous_period } |
| `scoring.scores` | scoring | read | — | Scores of a record in every active scoring model of its type, primary first: { record_type, record_id } -> [{ model_id, model_key, model_name, is_primary, score, band, warm_from, hot_from, explanation, calculated_at }] |
| `scoring.signal` | scoring | write | yes | Record a behavior signal on a record the actor can update and queue score recalculation; idempotent per record, signal and source: { record_type, record_id, signal: email_opened\|email_clicked\|email_replied\|meeting_booked\|form_submitted\|call_connected, source } -> { recorded } |
| `segments.get` | segments | read | — | One segment with its member count: { id } |
| `segments.list` | segments | read | — | Segments of the record types the actor can read: { record_type?, counts?, page?, per_page? } -> { data, total } |
| `segments.members` | segments | read | — | Records of a segment visible to the actor: { id, filter? (filter AST), sort? (e.g. -created_at,name), search?, archived?: exclude\|include\|only, page?, per_page? } -> { data, total } |
| `sequences.enroll` | sequences | write | yes | Enroll a contact in a sequence: { sequence_id, contact_id, mailbox_id? } |
| `sequences.exit` | sequences | write | yes | Remove a contact from a sequence (ends a live enrollment): { id } |
| `sequences.list` | sequences | read | — | Sequences with their steps and enrollment counts: { active?, search?, page?, per_page? } -> { data, total } |
| `sequences.pause` | sequences | write | yes | Pause an active enrollment: { id } |
| `sequences.resume` | sequences | write | yes | Resume a paused enrollment: { id } |
| `tagging.apply` | tagging | write | yes | Tag a record the actor can edit: { record_type, record_id, tag_id } -> { changed, tags } |
| `tagging.get` | tagging | read | — | One tag by id: { id } -> tag |
| `tagging.list` | tagging | read | — | Tags of the organization with their usage, optionally only those allowed on a record type: { record_type? } -> { data } |
| `tagging.remove` | tagging | write | yes | Remove a tag from a record the actor can edit: { record_type, record_id, tag_id } -> { changed, tags } |
| `tasks.complete` | tasks | write | yes | Mark an open activity as done now: { id } |
| `tasks.create` | tasks | write | no | Create a task: { title, activity_type_id, due_at?, owner_id?, record_type?, record_id?, description?, duration_minutes? } -> { id } |
| `tasks.types` | tasks | read | — | Active task types: {} -> { items: [{ id, key, name }] } |
| `telephony.log` | telephony | write | yes | Log a call as an activity: { outcome_key, record_type?, record_id?, direction?: inbound\|outbound, duration_seconds?, phone_number?, recording_url?, title?, notes?, occurred_at?, provider?, provider_call_id?, custom_fields? } -> the activity |
| `timeline.kinds` | timeline | read | — | Available timeline kinds and localized labels: { record_type, record_id } |
| `timeline.page` | timeline | read | — | Owner-authorized record timeline, newest first: { record_type, record_id, cursor?, kinds?: comma-separated contributed keys, per_page? } -> { data, meta: { next_cursor } } |
| `workflow.list` | workflow | read | — | Pipelines visible to the actor with their stages: {} -> { data } |
