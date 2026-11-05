# P5 acceptance run (2026-11-05)

Manual acceptance of the P5 checklists on isolated instances (own database, port and cookie), one agent per group, read-only against the repository. Boxes in the per-domain checklists are ticked only where this run produced evidence; lines marked "not runnable" need a code change, a real external provider or the web UI.

## Covered

| Group | Result |
| --- | --- |
| Assignment | Round-robin, weighted, routing, away windows, least-loaded workload, import/webhook/automation sources, package apply: pass. Repeated-source idempotency needs a job retry (not runnable). |
| Deals | Scopes, restricted pipelines, owner-reference edits, funnel/forecast/widget figures against SQL, rotting filters and catch-up: pass. Hidden party names and conflicting declarations: not runnable. |
| Workflow | Enable/disable isolation, move requirements, checklists, stage and pipeline removal with reassignment, stalled threshold catch-up: pass. |
| Money | Rates, rebase, field visibility, recalculation of open records, standalone enablement: pass. |
| Line items | Totals, limits, scopes, stale versions, recurring revenue, independent capability toggles, purge: pass. Custom-object attachment is not exposed (no `line_items.attachable` trait). |
| Documents, quotes, proposals | Quote and proposal PDFs, pagination, layouts, access checks, approvals, tiers and bundles: pass. Mail, public sharing and e-sign against real providers: not runnable. |
| Imports, webhooks, privacy | Run on a separate instance; its blocking defects are fixed below. |

## Defects fixed from this run

- Query engine rejected `sort=id`, which broke import lookups and webhook upserts; `id` is now an accepted sort target.
- Public booking failed with MySQL 1295 because `SAVEPOINT` ran as a prepared statement.
- `assign_by_rule` failed on `tasks.task` (no transactional update); the handler now implements it.
- Staged custom objects: stage counts ignored record scope and hidden stage fields, and the checklist returned 404.
- Database deadlocks surfaced as 500; they are now 409 `concurrent_modification`.
- Webhook assignment with the provider off returned 500 (now 422); mapping references must start with `$.payload` or point to an earlier `$.actions[n]`.
- Assignment targets and rule creation exclude entities owned by disabled modules.
- Privacy search covers custom and declarative objects with personal fields.
- Money and line item inputs reject more decimals than documented instead of rounding silently.
- Proposal items table shows a tax row; the quote PDF follows the organization locale.
- Re-enabling `deals` recalculates monetary records (rates changed while it was off were dropped).
- Public Google and Microsoft sign-in endpoints are rate limited.
- Earlier in the same run: automation `record.*` triggers for native entities, favorites limit locking, lifecycle history purge, hidden `due_at` in tasks.

## Fixed after the first pass

- Webhook assignment now creates the record inside the assignment transaction (`AssignmentSelector::create_assigned`): a failed create no longer advances the rule counter and the history row carries the record id.
- Changing the base currency converts the amounts of completed deals (`money.rebase_completed`, at most once); their line items are converted through `LineItemsPort::rebase_amounts`.
- A quote item with a rejected catalog product no longer reports a second "name is required" violation.
- Applying a configuration package that creates a custom object publishes `custom_object.created` with audit.

## Known and deferred

- A deal's `amount_base` is converted from its total and can differ by cents from the sum of its items' `amount_base` (per-line rounding); both are correctly rounded.
- A hidden catalog price on a quote still reports "product was not found" so that the catalog does not leak existence.
- Open: `p5-remaining-domains`, `p5-bridges`, P6 permission-refusal/cycle/`[[contribute]]` lines, P7 shortcut, palette and installer lines (need the web UI).
