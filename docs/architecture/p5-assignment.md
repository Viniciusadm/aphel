# P5.6 Assignment

`assignment` replaces `lead_distribution` for fresh installations. Its rules, state, counters and assignment history belong to assignment-owned tables. The SDK exposes the assignment permission and the pure `aphel-contract-assignment` workload/reference metadata.

## Static review

- [x] The capability accepts registered entities with an owner instead of enumerating people, organizations and deals.
- [x] The target API and rule editor use entity metadata and localized labels, including administrator entities. New rules require an explicit target.
- [x] Rule package validation uses registry ownership; unresolved administrator objects retain deferred validation so a package can create its objects before applying assignment rules.
- [x] Least-loaded selection queries the rule's target through the generic aggregate engine, groups by its owner field, limits candidates to the eligible users and excludes archived/deleted records.
- [x] Entity owners declare active workload states through `assignment.workload`. Deals and tasks declare their own open states. Entities without that mapping count their non-archived records.
- [x] Workload aggregation uses system scope for internal distribution statistics, preserving the previous global-load semantics. Counts do not expose records, and truncated aggregates fail rather than incorrectly assigning zero load.
- [x] Scheduling uses the same workload calculation rather than a deal-specific store method.
- [x] Round-robin, weights, routing, away periods, state/counter locking and atomic owner writes remain in the capability.
- [x] Assignment automation supports automatable entities with owners, including tasks, and keeps target update permission checks.
- [x] The assignment rule reference provider uses the canonical `assignment.rule` key.
- [x] No assignment query names a deal table.

## Manual acceptance pending

No runtime instance, automated test, probe or smoke run was executed under D4.

- [ ] Install a fresh schema and create rules for a native entity and an administrator object with an owner.
- [ ] Verify round-robin, weights, routing, away periods and repeated source requests under concurrent assignments.
- [ ] Verify least-loaded counts against the selected entity's active workload, including archived/deleted records and users without records.
- [ ] Simulate and execute the same rule through automation, imports, inbound webhooks and booking, checking assignment history and atomic audit/events.
- [ ] Apply a package that creates an administrator object and its assignment rule together.
- [ ] Check hidden fields, inaccessible targets, missing owners, invalid workload metadata and unavailable entity providers.

Existing native consumers still import assignment implementation types. They move to capability contracts/operations during the final P5 dependency cleanup.
