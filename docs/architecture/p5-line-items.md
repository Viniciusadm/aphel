# P5.9 Line items

`line_items` owns polymorphic record lines and their amount-mode settings. Its pure contract supplies quantities, recurrence, discounts, tax rates and checked decimal arithmetic. The native SDK supplies scoped line/summary, catalog and owner-projection ports. `deals_revenue` owns recurring-revenue policy and its widget; deals owns its financial fields, persistence, audit and events.

## Static review

- [x] Line rows use `(record_type, record_id)` throughout reads, mutations, ordering, conversion and purge cleanup. No foreign business-table FK remains in the line schema.
- [x] Parent entities opt in with `line_items.attachable`; currency, amount and conversion fields come from that declaration. Native and administrator entities use the same registry authorization and snapshot mechanism.
- [x] Reads enforce parent visibility and monetary field visibility. Mutations enforce the parent's update permission and writable amount fields for automatic totals.
- [x] Parent rows are locked before item writes, access is checked again, concurrent currency changes fail explicitly and conversions use the locked exchange rate. Item version checks remain inside the mutation transaction.
- [x] All item changes, owner projections, audit entries and events share a transaction. Automatic projections are contributed by owners; hosts without a projection retain informative lines and cannot apply totals to their parent.
- [x] Settings use a fresh singleton schema and lock/version-check that row before updating. No identity adoption, compatibility endpoint or data conversion is introduced.
- [x] Catalog lookup uses contributed owner adapters. Unavailable or ambiguous catalog identities do not select arbitrary records. Snapshot imports preserve valid references and otherwise retain free-line values.
- [x] Quote and line arithmetic share discount and line calculations. Generic lines support tax rates; quote transfers and renewals preserve taxes and recurrence.
- [x] Quote acceptance, proposal snapshots, demo data and renewals consume SDK ports instead of the line implementation crate. Replacing renewal lines is transactional.
- [x] Recurring revenue belongs to `deals_revenue`; deals owns writes to its table. Rate recalculation enumerates open records in their owner and updates lines through the SDK port.
- [x] Formula item relations include a declared type partition, preventing equal UUIDs in distinct parent types from mixing their totals.
- [x] The generic web section selects entities by trait, keys its cache by type and ID and uses a catalog-picker slot. Settings and item events belong to the capability; deal amount locks and recurring panels belong to their owners.
- [x] Widget interfaces and stateless query context helpers are exposed by the native SDK. Widget registry policy and persistence stay in dashboard; recurring-revenue providers require no dashboard implementation dependency. Further source extraction is P5.13.

## Manual acceptance pending

No tests, probes, database instance or runtime smoke run were executed, as required by D4.

- [ ] Exercise item CRUD, ordering and totals on a deal and a declared administrator entity, including equal parent UUIDs in different types.
- [ ] Verify parent scope, pipeline restrictions, hidden monetary fields, read-only amount fields, update-only tokens and stale item/settings versions.
- [ ] Check concurrent currency/rate changes and item edits, transaction rollback, line limits, overflow, discounts and tax precision.
- [ ] Transfer quote lines, generate proposal snapshots and renew recurring contracts with active, inactive and unavailable catalog entries.
- [ ] Verify MRR, ARR, TCV and contract dates in manual/automatic modes and widget periods, comparison, subject and field restrictions.
- [ ] Disable the capability, catalog provider and recurring-revenue provider independently; inspect settings, tabs, amount locks and missing-provider behavior.
- [ ] Purge a parent or remove an administrator entity and verify scoped line cleanup.

The repository-wide removal of remaining implementation dependencies and cross-plugin SQL is tracked separately by P5.22–P5.24. This item adds no baseline exception.
