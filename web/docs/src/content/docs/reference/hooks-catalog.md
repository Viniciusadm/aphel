---
title: Hooks catalog
description: Every hook that Aphel modules and runtime plugins can attach to, with its kind, arguments, the moment it fires and who listens to it today.
sidebar:
  order: 8
---

Hooks are the extension points where code can take part in an operation without the owner of that operation knowing about it. This page lists every hook of the build, grouped by area. The same catalog drives the `aphel catalog` and `aphel modules doctor` commands, and runtime plugins can read the live list, including whether the owning module is enabled, from `GET /api/v1/wasm-runtime/host-catalog`.

## Kinds of hook

| Kind | What a listener does | Effect |
| --- | --- | --- |
| `veto` | Inspects an operation that is about to happen, or that was written but not yet committed. | Returning an error cancels the operation, and the error reaches the caller. |
| `action` | Reacts to something that happened. | Runs for its effect. |
| `filter` | Receives a value and returns a changed one: the draft values of a record, an e-mail before sending, a query plan. | The changed value replaces the original. |

A hook with two kinds in the table (`veto, filter`) lets a listener do both. Listeners run in priority order, lowest number first. The platform and native modules use priorities such as 4, 5, 10, 20, 30 and 40; listeners that forward a hook to runtime plugins run at priority 50, after the native ones. Forwarding a hook to plugins costs nothing when no plugin subscribes.

Descriptions that say "inside the transaction" mean the hook runs within the database transaction of the change, so what it writes is committed or rolled back with it. `record.committed` runs after the commit and its errors are only logged.

## Subscribing from a plugin

A runtime plugin subscribes to hooks of this catalog in its manifest, and its backend answers each call. Which values arrive, what it may return and the limits (a short timeout, and no record writes inside hooks, which answer `409 plugin_write_in_hook`) are described in [Hooks](/docs/plugins/hooks/) and in [Manifest](/docs/plugins/manifest/). Operations that a plugin can call are a separate catalog, described in [Host API](/docs/plugins/host-api/).

A plugin sees only the hooks listed in the first part of this page. The typed hooks in the second part are Rust types used by native modules compiled into the binary; they are listed so that module authors know where to attach.

## Hooks for runtime plugins

These are the hook keys a plugin manifest can subscribe to. `record.*` hooks apply to every record type, the others to one record type.

### WebAssembly runtime

| Hook | Kind | Record type | Values and effect |
| --- | --- | --- | --- |
| `record.archiving` | veto | any | The custom record or company being archived |
| `record.committed` | action | any | `values` and `changes`, after the commit (errors only logged) |
| `record.deleted` | action | any | The deleted record, inside the transaction |
| `record.deleting` | veto | any | The record being deleted |
| `record.purged` | action | any | The purged record, inside the transaction |
| `record.restored` | action | any | The restored record, inside the transaction |
| `record.saved` | action | any | `values` and `changes`, inside the transaction |
| `record.saving` | filter | any | The draft `values`/`custom_fields`; return changes |
| `record.validating` | veto | any | `values` and `changes` of the write |

### Tasks

| Hook | Kind | Record type | Values and effect |
| --- | --- | --- | --- |
| `activity.completing` | veto | `tasks.task` | Before an activity is completed; values: { activity, previous, type_key }. An error blocks it |

### Automations

| Hook | Kind | Record type | Values and effect |
| --- | --- | --- | --- |
| `automation.action_running` | veto | any | Before an automation action runs on a record (dry run and execution), for the record's type; values: { automation_id, run_id, step_id, action, record: { record_type, record_id }, config, dry_run }. An error skips the action |

### Deals

| Hook | Kind | Record type | Values and effect |
| --- | --- | --- | --- |
| `deal.losing` | veto | `deals.deal` | Before a deal is lost; values: { deal, previous, stage, lost_reason }. An error blocks it |
| `deal.lost` | action | `deals.deal` | After a deal is lost, inside the transaction; values: { deal, previous, stage, lost_reason } |
| `deal.stage_changing` | veto | `deals.deal` | Before a deal moves to another stage; values: { deal, previous, from, to }. An error blocks the move |
| `deal.winning` | veto | `deals.deal` | Before a deal is won; values: { deal, previous, stage, lost_reason }. An error blocks it |
| `deal.won` | action | `deals.deal` | After a deal is won, inside the transaction; values: { deal, previous, stage } |

### Email

| Hook | Kind | Record type | Values and effect |
| --- | --- | --- | --- |
| `email.outgoing` | filter | `email` | Before an e-mail is saved and sent; record_type is the linked record's type (or email when unlinked); values: { mailbox_id, sender_id, from, to, cc, bcc, subject, body: { text, html }, record, thread_id, scheduled, tracking }. Return { subject?, body?: { text?, html? } } to change them (html is sanitized) |

### Leads

| Hook | Kind | Record type | Values and effect |
| --- | --- | --- | --- |
| `lead.converted` | action | `people.person` | After a lead was converted, inside the transaction; values: { lead, deal_id, company_id } |
| `lead.converting` | filter | `people.person` | Values of the deal (and optional company) a lead conversion creates; values: { contact_id, lead, deal, company }. Return { deal?: {..}, company?: {..} } to merge field values into them (a company object makes the conversion create a company when the contact has none) |
| `lead.disqualifying` | veto | `people.person` | Before a lead is disqualified; values: { lead, reason: { id, key, name }, note }. An error blocks it |
| `lead.qualifying` | veto | `people.person` | Before a lead is qualified in the lead inbox; values: { lead }. An error blocks the qualification |

### Quotes

| Hook | Kind | Record type | Values and effect |
| --- | --- | --- | --- |
| `quote.accepting` | veto | `quotes.quote` | Before a quote is accepted (public page or manually); values: { quote, deal_id, acceptor: { name, email }, manual }. An error blocks the acceptance |

### Sequences

| Hook | Kind | Record type | Values and effect |
| --- | --- | --- | --- |
| `sequence.step_running` | veto | `people.person` | Before a sequence step runs for an enrolled contact; values: { enrollment_id, sequence_id, contact_id, enrolled_by, position, kind, template_id }. A definitive error removes the contact from the sequence, a transient one retries the step |

## Hooks for native modules

These hooks are Rust types registered with `app.on` (veto and action) or `app.filter` (filter) in a module's `register` function. The key of a typed hook is its Rust path. The "Listeners" column shows the modules that listen today, with the kind and priority of each registration.

### Platform (`kernel`)

| Hook | Kind | Argument | When it fires | Listeners |
| --- | --- | --- | --- | --- |
| `aphel_kernel::records::hooks::RecordArchiving` | veto | `RemovedRecord` | Before a record is archived; an error blocks the archive | wasm_runtime (action 50) |
| `aphel_kernel::records::hooks::RecordCommitted` | action | `ChangedRecord` | After the transaction that wrote a record was committed | - |
| `aphel_kernel::records::hooks::RecordDeleted` | action | `RemovedRecord` | After a record was deleted, inside the same transaction | leads (action 40), wasm_runtime (action 50) |
| `aphel_kernel::records::hooks::RecordDeleting` | veto | `RemovedRecord` | Before a record is deleted; an error blocks the deletion (restrict relationships) | kernel (action 10), wasm_runtime (action 50) |
| `aphel_kernel::records::hooks::RecordPurged` | action | `RemovedRecord` | After a record was permanently deleted (trash purge, erasure, retention), inside the same transaction, before the kernel removes its custom values, relationship links, attachments, shares and the record fields that pointed to it | deals (action 10), duplicates (action 10), line_items (action 10), messaging (action 10), notes (action 10), people (action 10), pricing (action 10), quotes (action 10), renewals (action 10), sequences (action 10), tagging (action 10), tasks (action 10), telephony (action 10), workflow (action 10), leads (action 40), wasm_runtime (action 50) |
| `aphel_kernel::records::hooks::RecordQueryFilter` | filter | `QueryConstraints` | Extra AND conditions and sorting applied to every query of a record type | deals (filter 10) |
| `aphel_kernel::records::hooks::RecordRestored` | action | `RemovedRecord` | After a deleted record was restored from the trash, inside the same transaction, after RecordValidating and before RecordSaved | leads (action 40), wasm_runtime (action 50) |
| `aphel_kernel::records::hooks::RecordSaved` | action | `ChangedRecord` | After a record was written, inside the same transaction | imports (action 10), wasm_runtime (action 50) |
| `aphel_kernel::records::hooks::RecordSaving` | filter | `SavingRecord` | Values and custom fields of a record before they are written (create and update of every record type) | wasm_runtime (filter 50) |
| `aphel_kernel::records::hooks::RecordSchemaFilter` | filter | `RecordSchema` | The field schema of a record type returned to forms, lists and filters | - |
| `aphel_kernel::records::hooks::RecordSerializing` | filter | `SerializedRecords` | Records before they are returned by the API (after the field permissions) | tasks (filter 10), telephony (filter 10) |
| `aphel_kernel::records::hooks::RecordValidating` | veto | `ChangedRecord` | After a record was written and before the commit; an error rolls the write back (approvals, stage checklists, validation rules, lifecycle) | line_items (action 4), workflow (action 5), approvals (action 10), workflow (action 20), validation_rules (action 30), leads (action 40), wasm_runtime (action 50) |
| `aphel_kernel::records::hooks::SearchPlanFilter` | filter | `QueryPlan` | The query plan of the global search for one record type (extra restrictions) | deals (filter 10) |
| `session.revoked` | action | `SessionRevocation` | A session or a user session set was revoked in the current transaction | notifications (action 10) |

### Automations (`automations`)

| Hook | Kind | Argument | When it fires | Listeners |
| --- | --- | --- | --- | --- |
| `automations::hooks::ActionRunning` | veto, filter | `RunningAction` | Before an automation action runs (dry run and execution; an error skips it) and, as a filter, the result recorded for the step | automations (action 50) |

### Deals (`deals`)

| Hook | Kind | Argument | When it fires | Listeners |
| --- | --- | --- | --- | --- |
| `deals::hooks::DealLosing` | veto | `ClosingDeal` | Before a deal is marked as lost; an error blocks it | deals (action 50) |
| `deals::hooks::DealLost` | action | `ClosingDeal` | After a deal was lost, inside the same transaction | deals (action 50) |
| `deals::hooks::DealStageChanging` | veto | `StageChange` | Before a deal moves to another stage; an error blocks the move | deals (action 50) |
| `deals::hooks::DealWinning` | veto | `ClosingDeal` | Before a deal is marked as won; an error blocks it | deals (action 50) |
| `deals::hooks::DealWon` | action | `ClosingDeal` | After a deal was won, inside the same transaction | deals (action 50) |

### Email (`email`)

| Hook | Kind | Argument | When it fires | Listeners |
| --- | --- | --- | --- | --- |
| `email::hooks::InboundLinking` | filter | `LinkingEmail` | The records an incoming e-mail is linked to automatically by the mailbox synchronization | - |
| `email::hooks::OutgoingEmail` | filter | `OutgoingMessage` | An e-mail before it is saved and sent; without recipients or subject the send fails with 422 | email (filter 50) |

### Leads (`leads`)

| Hook | Kind | Argument | When it fires | Listeners |
| --- | --- | --- | --- | --- |
| `leads::hooks::LeadConverted` | action | `ConvertedLead` | After a lead was converted, inside the conversion transaction | leads (action 50) |
| `leads::hooks::LeadDisqualifying` | veto | `DisqualifyingLead` | Before a lead is disqualified with a reason; an error blocks it | leads (action 50) |
| `leads.converting` | filter | `LeadConversion` | Values of the deal (and optional company) created by the lead conversion; return them changed to enrich the conversion | leads (filter 50) |
| `leads.qualifying` | veto | `Lead` | Before a lead is qualified in the lead inbox; an error blocks the qualification | leads (action 50) |

### Quotes (`quotes`)

| Hook | Kind | Argument | When it fires | Listeners |
| --- | --- | --- | --- | --- |
| `quotes::hooks::QuoteAccepting` | veto | `AcceptingQuote` | Before a quote is accepted (on its public page or manually by a user); an error blocks the acceptance | quotes (action 50) |
| `quotes::hooks::QuoteTotals` | filter | `PricedQuote` | Totals of a quote (items, discounts and taxes) before they are saved | - |

### Sequences (`sequences`)

| Hook | Kind | Argument | When it fires | Listeners |
| --- | --- | --- | --- | --- |
| `sequences::hooks::StepRunning` | veto | `RunningStep` | Before a sequence step runs; a definitive error removes the contact from the sequence, a transient one retries the job | sequences (action 50) |

### Tasks (`tasks`)

| Hook | Kind | Argument | When it fires | Listeners |
| --- | --- | --- | --- | --- |
| `tasks::hooks::ActivityCompleting` | veto | `CompletingActivity` | Before an activity is completed; an error blocks it | tasks (action 50) |

## Reading the listeners

For example, `leads (action 40)` on a hook means the `leads` module registered an action listener at priority 40, and `wasm_runtime (action 50)` is the bridge that forwards the hook to runtime plugins. A hook with `-` as its listener has none in the default build: it is an extension point waiting for a module or plugin.
