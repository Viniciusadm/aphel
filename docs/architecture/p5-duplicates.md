# P5 duplicates ownership

## Implementation review

- [x] Duplicates has no people or organizations implementation dependency.
- [x] SDK merge contracts contain interfaces, opaque prepared receipts and pure field-choice logic; no SQL or owner business engines.
- [x] People owns typed contact copying, empty-field rules, primary organization preservation and extra organization associations.
- [x] Organizations owns typed company copying, hierarchy locking, cycle exclusion and child reparenting with its native audit/events.
- [x] Catalog items owns typed product copying; every currently declared native duplicate identifier type has an owner handler.
- [x] Owner receipts persist survivors after retirement and custom writes; active product codes are released before copying.
- [x] Tasks, notes, tagging, deals and people own reference updates and deduplication; maintenance includes disabled compiled owners, skips entirely absent schemas and rejects partial schemas.
- [x] Core helpers own attachments, typed custom references and typed relationship deduplication; custom entity column copying and retirement reject native definitions.
- [x] Duplicates retains scoped access, version comparison and locking, custom-field choice validation, hidden-field redaction, record pipeline hooks, lifecycle history, audit/events, tombstones and commit ordering.
- [x] Remaining formatted duplicates SQL reads registry-defined records for detection, visibility, versions and custom merge snapshots; no foreign owner writes use formatted SQL.
- [x] Owner-specific missing-record errors remain owner-defined.

## Validation

- [x] Scoped cargo check: duplicates, people, organizations, catalog-items, tasks, notes, tagging and deals.
- [x] Scoped formatting and whitespace review.
- [ ] Manual acceptance: contact field choices, hidden fill-empty behavior, extra organization associations and deduplicated references.
- [ ] Manual acceptance: organization parent/child reparenting, cycle rejection and hierarchy audit.
- [ ] Manual acceptance: product code selection, custom entity merges, lifecycle history, stale versions and merge redirects.
- [ ] Manual acceptance: disabled owner maintenance, absent schemas and partial-schema rollback.
- [ ] P5.22 full workspace lint and architecture ratchet, serialized by the phase coordinator.
