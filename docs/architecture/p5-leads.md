# P5.20 Leads through owner handlers

Leads owns lifecycle fields/extensions, stage history, transition rules, disqualification reasons and lead actions. People/organizations own their records; deals owns commercial-party/outcome lookup. Lead conversion coordinates registered handlers inside one transaction.

## Static review

- [x] Leads no longer depends on people, organizations or deals implementation crates. Lead hooks carry scoped record snapshots rather than a people implementation struct.
- [x] Qualify/disqualify/conversion changes use transactional owner handlers. Shared saving/validation/saved hooks, field policies, owner normalization, audit and outbox writes remain in the owner pipeline.
- [x] Conversion locks the declared record version through the generic core primitive and preserves transactional rollback across optional organization/deal creation and the lifecycle update.
- [x] Core snapshot record versions preserve microseconds; generic owner updates receive the standard `_expected_updated_at` token. Missing readable versions reject the action, and hidden names use identity-only fallback labels.
- [x] Ctx exposes scoped access checks and transactional record operations. Native record operations reject disabled declared owners before invoking their handlers.
- [x] Deal lifecycle identities/outcome data and event keys belong to the deals contract/SDK. Deals implements its own SQL source; missing/disabled sources do not invent zero outcomes.
- [x] Lifecycle reports/history query leads-owned tables and use authorized identity subqueries composed by the core. Direct SQL over people, organizations and deals was removed.
- [x] Lifecycle report/history access respects the lifecycle field's visibility and native owner availability. Record scopes and constraints are included in parent identity subqueries.
- [x] Lifecycle fields are contributed through a registry transform, so absent parent plugins do not require extending an unknown native definition. Each extension table remains owned by leads.
- [x] The fresh schema creates extension tables directly in migration 1. The column-copy/adoption migration and prepared information-schema checks were deleted.
- [x] The foreign disqualification/person FK was replaced by leads-owned cleanup on the generic purge hook. The disqualification/reason FK remains within its owner.
- [x] Lead forms consumes the scoped lead snapshot API; existing converter and plugin-hook call sites compile with the new model.

## Manual acceptance pending

D4 prohibits tests, probes and runtime instances. Verification uses compilation, strict lint, frontend checks/build and CLI API/catalog generation.

- [ ] Qualify/disqualify/convert under own/team scopes, hidden fields, unreadable record versions, disabled owners and inactive disqualification reasons.
- [ ] Race owner updates with lead conversion; inspect version conflicts, veto/filter hooks, created-organization/deal rollback and post-commit processing.
- [ ] Inspect automatic deal-created/won/lost/deleted lifecycle transitions, absent providers, rule eligibility and concurrent manual stage changes.
- [ ] Compare lifecycle history/report counts for archived, deleted, restored and scoped parent records, including inactive stage options and hidden lifecycle fields.
- [ ] Inspect fresh installations with optional parent plugins, transformed extension fields, purge cleanup and disabled cleanup providers.
- [ ] Inspect lead forms, lead-pro conversion filters and runtime plugin hook payloads in both locales.

Shared configuration utilities and transitional CRM vocabulary remain in P5.22–P5.24. The complete foreign-reference/deletion audit remains in P5.24. Runtime acceptance has not been performed.
