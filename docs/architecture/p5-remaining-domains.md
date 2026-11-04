# P5.22 remaining domain plugins

This phase is in progress. Completed entries describe static source validation; manual acceptance remains pending. The main P5.22 checkbox stays open until all remaining domain plugins have been reviewed.

## Booking identity

- [x] Replace the native module, Cargo package/feature, manifest, permissions, event/job namespaces and web module identity with `booking`.
- [x] Use `/booking` for authenticated screens/API and preserve the intentional public `/book` experience.
- [x] Rewrite the fresh availability schema and every consumer to use `booking_availability`; no copy migration or old identity alias.
- [x] Update assignment source metadata, profiles, generated registries, translations and API consumers.
- [ ] Remove remaining task, party, assignment and email implementation dependencies and foreign SQL.
- [ ] Manual acceptance: availability editing, meeting types, public booking, host assignment, cancellation, rescheduling and email delivery with module/permission combinations.

## Forms identity

- [x] Replace the native module/package/feature, HTTP namespace, permission namespace and web module with `forms`.
- [x] Rewrite fresh form/submission tables, every SQL consumer, manifests, profiles, documentation and generated registries without a legacy identity alias.
- [x] Forms uses enabled tagging contract operations for options, validation and assignment; provider failures propagate instead of appearing as an empty option catalog.
- [ ] Replace remaining fixed CRM targets and peer implementation imports with owner interfaces and entity metadata.
- [ ] Manual acceptance: settings/editor, publish/embed/public render, submission validation, spam controls, mapping and downstream actions.

## Tagging identity

- [x] Replace the native module/package/feature, web namespace and settings route with `tagging`; HTTP resources use `/tagging` and `/tagging/records`.
- [x] Align record identity with the existing `tagging.tag` contract. Preserve the logical `tags` field, package section and explicitly owned `tags`/`taggings` tables.
- [x] Remove the legacy reference-namespace conversion migration; fresh installs already write current reference keys.
- [x] Replace entity response coupling with generic SDK include sources and remove the fixed CRM tagging target list. Owner metadata and permissions drive the settings and record controls; custom records remain supported.
- [ ] Manual acceptance: tag management, record lookup/includes, assignment/removal, search refresh, saved tag filters and portable tag configuration.

## Generic response includes

- [x] Add a bounded SDK `RecordIncludeSource` extension point. Sources own their lookup, visibility and wire data; the generic host validates requested keys, rejects duplicate sources/unrequested IDs and prevents overwriting existing response fields.
- [x] People and organizations enrich redacted responses through the generic include interface. They no longer import tagging DTOs/services or declare tag fields.
- [x] Tagging supplies the `tags` include with scoped, chunked owner queries. Native disabled record owners are rejected by record tagging/lookup actions.
- [x] Generic automation bindings select required entity metadata rather than excluding a hardcoded task entity. Tagging's declaration explicitly supports custom records.
- [x] Configuration administrators can discover enabled entity schemas with explicit access flags; this grants no record read/write access. Settings use this metadata and block saving while the type catalog is unavailable.
- [ ] Manual acceptance: custom and native opt-in entities, unknown includes, duplicate/colliding include sources, hidden/out-of-scope records, merged record IDs, disabled owners, schema-only administrators and metadata load failures.
- [ ] Runtime/managed owner identity checks remain part of the final audit; the native enabled-owner guard does not establish runtime owner parity.

## Pricing identity

- [x] Replace the native module/package/feature, permission namespace, record identity and web namespace/routes with `pricing`.
- [x] Serve price lists under `/pricing`, discount limits under `/pricing/discount-limits` and bundles under `/pricing/items/{product_id}/bundle`.
- [x] Preserve the explicitly owned price-list/bundle tables and logical configuration section names; update all consumers and manifests without compatibility aliases.
- [ ] Remove remaining peer implementations, foreign SQL and fixed party/catalog coupling.
- [ ] Manual acceptance: lists/items, limits, bundle save/delete/expansion, settings links and quotation price resolution.

## Telephony identity

- [x] Replace the native module/package/feature, API/web namespace and native outcome/session record identities with `telephony`.
- [x] Keep explicitly owned call tables and call activity/event semantics. Update recording/event/session URLs, query-cache invalidation, providers, profiles and registries without an old module alias.
- [ ] Replace remaining task implementation imports, owner-only SQL and foreign task/view foreign keys; shared provider interfaces are still pending.
- [ ] Manual acceptance: call logging/update, recordings, provider events/start, outcomes, reports, sessions/queues and offline invalidation.

## Catalog item identity

- [x] Replace the native module/package/feature and web namespace with `catalog_items` (`catalog-items` for Cargo/HTTP paths).
- [x] Use `catalog_items.item` as record identity and `catalog_items.*` permissions; update registered handlers, import/export/bulk metadata, relationships, profiles, labels and consumers.
- [x] Keep the explicitly owned `products` table and product payload semantics, with no legacy record/permission/transport alias.
- [x] Catalog items consumes the shared money input codec directly and has no peer implementation dependency.
- [ ] Complete foreign reference policies in the owning consumers.
- [ ] Manual acceptance: item create/edit/delete, picker and detail paths, role permissions, fields/settings labels, import/export/bulk, pricing/bundles and quote catalog selection.

## Lead Pro fresh schema

- [x] Remove the legacy temperature-copy migration. The initial schema already creates the owned extension directly.
- [ ] Remove peer implementations and foreign party SQL; review behavior and manual acceptance.

## Notification extension boundary

- [x] Publish kernel-free notification kinds/channel metadata through `aphel-contract-notifications`.
- [x] Define native source, presentation, digest/follower, directory and delivery interfaces in the SDK. Owners contribute their own event policies and localized text.
- [x] Keep notification inserts, preference evaluation, deduplication, email/push queuing and delivery in `notifications`.
- [x] Replace peer implementation imports with SDK interfaces, including event/job consumers and the runtime notification adapter. Delivery resolves the enabled service at execution time; missing delivery is an explicit unavailable error.
- [x] Move generic mention parsing to SDK messaging utilities; note limits remain with notes. Generic local-day calculations belong to SDK calendar utilities; reminder schedules remain with reminders.
- [ ] Manual acceptance: source enable/disable, field/record visibility, recipient preferences, dedupe, actor suppression, localized mail, digest timezone/DST, reminders and runtime plugin notification capability checks.
- [ ] Review pending jobs when notification delivery is disabled and storage/queue failures; static validation does not establish retry or delivery acceptance.

## Configuration section boundary

- [x] Publish portable package, issue, decision and plan metadata in the kernel-free configuration contract. Header validation receives the installation's supported locales explicitly.
- [x] Define section factories, key lookup/write ports and version fragment layout contributions in the SDK.
- [x] Share native schema projection, field metadata and configuration-key helpers used by section contributors. Field apply operations and formula checks stay in the host.
- [x] Keep SQL key access, install/version repositories, installation planning/apply, template loading and version algorithms in `config_packages`.
- [x] Replace implementation dependencies for configuration section contributors with the shared boundary.
- [ ] Replace marketplace's remaining direct versioning consumers.
- [ ] Manual acceptance: export/plan/apply for each contributed section, stable key resolution, conflicts and decisions, custom schema projections, formula validation, disabled sections, package versions and failed transactional writes.

## Export utility and job boundary

- [x] Publish export format and outcome notification identifiers through a pure contract.
- [x] Share generic CSV/XLSX encoding and generated-file storage helpers through the SDK. Export record selection, column policy, limits and job persistence remain in `exports`.
- [x] Queue bulk record exports through the enabled `ExportJobs` service, retaining the owner's authorization and validation.
- [x] Remove export implementation imports from imports, privacy, reports, bulk actions and organization export. Imports/privacy/organization exports no longer require the queued record-export plugin.
- [x] Privacy and organization export contribute their own outcome text and kind metadata. Notification text adapters distinguish their data from a record-export job and use the appropriate action label.
- [ ] Manual acceptance: export scopes and hidden columns, queued job progress/download, CSV formula neutralization and round-trip imports, XLSX limits, error reports, generated-file failures and privacy/organization notifications with record exports disabled.

## Search and dashboard extension consumers

- [x] Use the existing SDK dashboard widget/context/port interfaces directly in goals, forecast, insights, renewals and runtime widgets. The host retains widget execution, layout and SQL repositories.
- [x] Define native search content source/refresh interfaces in the SDK. Notes and tagging retain their owned content queries; the search host retains indexing, source aggregation and permission filtering.
- [x] Share the existing pure name-normalization helper without moving search query validation or indexing policy.
- [x] Remove replaced and stale implementation dependencies, including privacy's unused search dependency, workflow's configuration dependency and renewals' unused catalog dependency.
- [x] Organization export consumes enabled SDK search maintenance for rebuilding or scheduling the index after restore; search retains the index engine and job protocol.
- [ ] Manual acceptance: widget registration/filtering/settings, runtime widgets, note/tag index refresh, source permission filtering, accent normalization and owner enable/disable.

## Saved view sharing and read boundary

- [x] Publish shared visibility/view-type values in the pure views contract, with native wire schemas in the SDK. Reports no longer imports saved-view implementation types.
- [x] Read call-session view definitions through an enabled SDK reader. Saved views owns persistence, private/team/organization visibility and parent-entity read/enable checks.
- [x] Keep saved-view filters, sorting and settings as snapshot data; query execution and call-session behavior remain in their respective owners.
- [ ] Remove telephony's remaining task dependency, foreign SQL and foreign view/task foreign keys.
- [ ] Manual acceptance: report sharing, view schema compatibility, private/team/organization views, disabled parent owners, call-session selection/sorting/settings and hidden or deleted views.

## Shared money input consumers

- [x] Catalog items, pricing and goals use the SDK money input codec directly; no DTO is imported through deals. Monetary parsing and endpoint schemas are unchanged.
- [ ] Manual acceptance: numeric/string inputs, invalid precision and amount validation on these screens.

## Optional WebAssembly host identity

- [x] Rename the native host to `wasm_runtime`, its crate to `aphel-mod-wasm-runtime` and its HTTP/settings paths to `wasm-runtime`, without legacy transport aliases.
- [x] Remove the host's required flag. Installer profiles explicitly select it; the generic optional-module Cargo feature remains unchanged.
- [x] Register runtime management settings through the host's own web module. Keep generic plugin package protocols, owned storage and application routes unchanged.
- [x] Create managed objects through an enabled SDK writer; validation, metadata persistence, audit and events remain in objects. The host has no peer implementation dependency.
- [ ] Review managed entity availability when the host is disabled or absent.
- [ ] Manual acceptance: installation, grants, settings, bundles, public routes, change streams, queued effects, module disabling and host-free composition.

## Telephony provider boundary

- [x] Publish call event identifiers through a pure telephony contract and native provider/request/recording/router interfaces through the SDK.
- [x] Remove scoring's telephony implementation dependency; its event matching uses shared identifiers.
- [x] Keep provider selection in telephony. Resolve contributions on each lookup, filter by captured owner availability and actor scope, and retain provider-specific module restrictions.
- [x] Remove the one-time provider cache so replaced runtime contributions do not leave stale provider instances.
- [ ] Remove remaining task implementation coupling and foreign SQL/FKs.
- [ ] Manual acceptance: provider registration/replacement, owner disable/enable, provider restrictions, start/recording failures and call event processing.

## Split-credit goal metrics

- [x] Share credit target/total snapshot data and a scoped read interface in the native SDK. Deals retains query planning, field visibility, credit arithmetic and all SQL.
- [x] Goals resolves an enabled credit reader; an unavailable reader for this metric produces absent progress, matching other unavailable metrics.
- [x] Remove goals' deals implementation dependency and make the capability optional.
- [ ] Manual acceptance: user/team split credit, owner remainder, read scopes, hidden amount fields, date boundaries and disabled deals.

## Onboarding configuration references

- [x] Read template existence and installation references through an enabled configuration catalog interface in the SDK.
- [x] Configuration packages retains template loading, install repository SQL and settings permission enforcement.
- [x] Remove onboarding's configuration implementation dependency and foreign install-table SQL. Completion without a template remains available when configuration packages is absent.
- [ ] Manual acceptance: selected templates, matching/mismatching installations, missing catalog, invalid identifiers, completion and reopening.

## Configuration package execution boundary

- [x] Share portable export/apply commands and plan metadata through the pure configuration contract, and installation snapshots through the SDK.
- [x] Execute package export, planning and apply through an enabled SDK interface. The owner retains validation, fingerprints, decisions, transactions, repositories and section execution.
- [x] Demo and organization export no longer construct the configuration implementation or depend on its crate. Missing package execution returns an explicit unavailable error.
- [ ] Replace marketplace's direct versioning implementation dependency.
- [ ] Manual acceptance: demo template setup, full configuration export/restore, conflict decisions, fingerprints, failed writes and disabled configuration packages.

## Managed object creation boundary

- [x] Share managed creation commands and a writer interface in the native SDK. Objects retains administrative validation, transactions and publication.
- [x] WebAssembly schema apply resolves the enabled writer before changing entity schemas; no object implementation is constructed by the host.
- [ ] Manual acceptance: managed entity installation, required title fields, permissions, metadata conflicts, schema failures and disabled object administration.

## Search maintenance boundary

- [x] Expose native rebuild results and maintenance interface through the SDK; index queries and the reindex job protocol remain with search.
- [x] Remove organization export's search implementation dependency. Restores do not queue unavailable search handlers when the capability is disabled.
- [ ] Manual acceptance: CLI restore rebuild, installer queued rebuild, disabled search, indexing failure and queue failure after restore.

## Outstanding scope

The remaining identity changes, shared capability interfaces, owner-only SQL, deletion policies and per-plugin manual acceptance matrices are still pending. Existing architectural debt is not considered resolved by renaming packages.
