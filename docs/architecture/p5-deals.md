# P5.19 Deals and owner-provided metadata

Deals owns commercial records, transitions, membership, revenue projections and commercial reports/widgets. Workflow owns stage metadata, restrictions, requirements and computed stage-age definitions. Party titles are resolved through the generic scoped record engine.

## Static review

- [x] Deals has no implementation dependency on workflow, people or organizations. Shared pipeline/stage/lost-reason snapshots and stage-kind data belong to the workflow contract; native metadata access uses the SDK WorkflowCatalog capability.
- [x] Workflow supplies its own catalog implementation and stage reference provider. The reference provider is available independently of deals.
- [x] Stage/team visibility rules and shared wire projections are consumed through the SDK instead of workflow HTTP/store implementation imports.
- [x] The deal read store queries only deals and core users. Related titles are batch-resolved with current scopes, enabled owners, storage partitions, archived-record support and title-field policies.
- [x] Shared record-label resolution no longer captures full privacy snapshots or locks records. It queries scoped identities in a bounded batch and decodes the declared title field through the core field decoder.
- [x] Deal party-reference edits require visible records through the same generic engine; hidden titles keep identity-only labels.
- [x] Commercial reports join deal history with bound workflow metadata snapshots, rather than reading workflow tables. Funnel, latest-open-stage loss and duration queries retain their history/window semantics.
- [x] Dashboard forecasts aggregate owned deal rows and use owner-supplied stage probabilities. Stale-deal labels use batched stage metadata rather than foreign joins.
- [x] Workflow contributes computed stage-age/rotting fields through a generic registry transform. Deals opts in declaratively; workflow owns the SQL for its own stage table. Active-state literals are encoded rather than interpolated as quoted SQL text.
- [x] Registry transforms apply after native extensions and to subsequently registered definitions; generated fields retain their contributing owner's availability metadata. Duplicate transform keys and conflicting computed fields reject configuration.
- [x] Dashboard ports receive the enabled service registry so owner widgets can discover native capabilities without constructing application state.

## Manual acceptance pending

D4 prohibits tests, probes and runtime instances. Verification uses compilation, strict lint, type checks/build and CLI API/catalog generation.

- [ ] Inspect deal list/detail/board/create/update/transitions with hidden party names, absent party owners, own/team scopes, restricted pipelines and disabled workflow.
- [ ] Inspect owner-reference edits with inaccessible, archived, deleted and concurrently changed targets.
- [ ] Compare commercial funnel, losses, stage durations, cycle times, forecast and stage widgets over open/won/lost records, reordered/inactive stages, pipeline moves and history ties.
- [ ] Inspect rotting filters, trigger catch-up, active-state values, mapped system columns and conflicting field declarations.
- [ ] Inspect native/managed registration and registry refresh, including providers absent or disabled after registration. Rotting is an explicit opt-in and requires declared system fields for its mapped values.
- [ ] Inspect pagination and large bounded record-label batches, title types, hidden fields and query constraints in both locales.

Fresh-install foreign-key removal and declarative deletion/reference policies remain in P5.24. Notification/configuration utilities, transitional CRM vocabulary and the remaining frontend cross-plugin imports are tracked in P5.22–P5.24. Runtime acceptance has not been performed.
