# P5.11 Privacy sources

Privacy coordinates subject discovery, export, erasure and retention through entity metadata and owner-provided sources. The pure privacy contract owns subject/action identities, source targets and entity policies. Native source interfaces, subject matching and redaction utilities belong to the SDK.

## Static review

- [x] The host no longer queries notes, tasks, import rows, duplicate candidates or webhook payload tables. Each owner contributes its own discovery, snapshot, erasure, text-redaction or inactivity policy.
- [x] Source keys and opaque item identities are validated. Native task selections become canonical entity targets. Persisted malformed targets fail explicitly instead of silently losing items.
- [x] Source progress is claimed transactionally with owner writes and audit/outbox publication. Retries skip committed source work; filesystem cleanup follows commit.
- [x] The generic entity engine owns declared-table matching, snapshots, lifecycle changes and erasure. Trash also consumes its purge primitive. The kernel contains no privacy source or business-text policy.
- [x] Personal fields, generated/read-only fields, protected owner fields and recency mappings come from entity declarations. Partitioned dynamic records retain their declared partition; custom values retain their entity/field identity.
- [x] Retention uses the scoped generic query compiler. Notes and tasks supply their own recent-activity exclusions rather than foreign SQL in the host.
- [x] Search, request selections, export sections and reports use open source identities. The interface discovers source labels and retention entities from an authenticated, non-cacheable catalog.
- [x] Privacy management remains mandatory. Search terms stay in request bodies; history records counts rather than raw terms. Disabled native contributors remain available for retained-data handling when their tables exist.
- [x] WebAssembly subject-source interfaces moved from kernel to SDK. Email consumes shared masking without importing the privacy implementation.
- [x] No identity aliases, compatibility transports or data-adoption migrations were introduced.

## Manual acceptance pending

D4 prohibits automated tests, probes and runtime instances. Compilation, strict lint, type checking and frontend build validate this change statically. OpenAPI generation constructs the route registry without starting a server or connecting to the database.

- [ ] Discover/export/anonymize/delete native, managed and partitioned custom records with optional, required, generated and sensitive personal fields.
- [ ] Select mixed source items, including import IDs containing colons, and inspect localized labels, request details and per-source reports.
- [ ] Verify source retries after commit, partial failures, unavailable owners and orphan-file cleanup failures.
- [ ] Inspect note privacy, task links, duplicate cleanup, webhook payload redaction and retained data from disabled contributors under privacy.manage.
- [ ] Inspect recent notes/tasks and owner-declared recency fields when previewing and running retention.
- [ ] Exercise uploaded originals, generated error files, active-import coordination and atomic native-source rollback implemented in P5.16; see [the import review](p5-imports.md).
- [ ] Inspect external plugin sources and export package limits, attachment handling and expiry.

Privacy still consumes notification, search and export implementation utilities; their shared capability boundaries are part of P5.22–P5.24. This step removes the host's foreign business-table SQL, not all remaining repository dependencies or all product privacy limitations.
