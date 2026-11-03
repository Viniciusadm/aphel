# P5.16 Entity-driven imports and owned import data

Imports owns upload parsing, mapping, execution history and row outcomes. Field aliases, exclusions, required-field alternatives and label-based reference creation are declared by entity owners through the pure imports contract and SDK. Writes continue through registered entity handlers.

## Static review

- [x] The importer has no fixed CRM alias table or business-field blacklist. People, organizations, deals and catalog products own their import metadata; other declarations can provide the same trait data.
- [x] Importability and action permissions come from the current entity registry and owner availability. Native, managed and dynamic targets use the same path; dynamic title fields are no longer excluded by a custom-type branch.
- [x] Reference-label and duplicate lookup use the scoped query engine rather than SQL assembled over foreign entity tables. Field permissions, entity constraints and storage partitions apply through that engine.
- [x] Typed system values, including booleans and lists, are preserved in write bodies. Mapped relationship writes also reach the registered update handler.
- [x] Reference creation is owner-declared and permission-checked. The API option is `create_missing_references`, with no old-name alias.
- [x] Assignment selection uses an optional SDK capability implemented by assignment. Imports no longer depends on that implementation crate; the shared web slot uses the assignment namespace.
- [x] The web UI derives import actions, target choices, labels, permissions and return links from entity/resource metadata instead of fixed record-type lists. The generic entity catalog includes create-only access metadata.
- [x] Import privacy discovery includes raw cells, file names, headers and errors. Selected rows use valid empty arrays; strong identifier mentions in other rows are redacted without changing unrelated values.
- [x] Affected import originals/error-file references are removed, previews cleared and identifying job metadata replaced. Core file cleanup deletes only unreferenced file rows and returns storage keys for deletion after commit.
- [x] Privacy-redacted jobs have an explicit timestamp and nullable original file ID. They cannot be edited, resumed or retried; transactional save guards reject stale requests that raced with redaction.
- [x] Queued/running affected imports reject erasure before their job state is changed. Native privacy-source writes, receipts and audit/outbox events now share one transaction, so a later native source failure rolls back earlier source erasures.
- [x] Fresh-install schemas were rewritten directly. No adoption migration, legacy identity alias or compatibility transport was introduced.

## Manual acceptance pending

D4 prohibits tests, probes and runtime instances. Verification uses compilation, strict Clippy/lint, type checks, frontend build and metadata generation.

- [ ] Upload CSV/XLSX sources in both locales over native, managed and dynamic entities; inspect aliases, required fields, booleans, lists, dates and relationship updates.
- [ ] Verify lookup and duplicate matching under own/team scopes, hidden fields, inactive/deleted references and partitioned storage.
- [ ] Inspect reference creation and assignment with disabled/missing providers, insufficient permissions and retry idempotency.
- [ ] Inspect create-only roles, target selection, history, labels and resource links without CRM modules installed.
- [ ] Export/anonymize/delete import artifacts; inspect matched rows, strong identifier mentions, headers, previews, error files, original downloads and shared file references.
- [ ] Race privacy processing with draft start/configuration, queued/running jobs, failed-row retry and completion; verify atomic native-source rollback and that redacted jobs cannot write again.
- [ ] Inspect object-store deletion failures and retry/cleanup behavior. Database reference removal does not establish removal of unrelated external copies or guarantee successful storage I/O.

The P5.11 import file-lifecycle limitation is implemented here; runtime acceptance remains pending. Imports still consumes notification and export implementation utilities, and the migrations assistant still consumes import implementation APIs. Their shared capability boundaries remain in P5.22–P5.24.
