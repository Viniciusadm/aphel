# P5.22 remaining domain plugins

This phase is in progress. Completed entries describe static source validation; manual acceptance remains pending. The main P5.22 checkbox stays open until all remaining domain plugins have been reviewed.

## Booking identity

- [x] Replace the native module, Cargo package/feature, manifest, permissions, event/job namespaces and web module identity with `booking`.
- [x] Use `/booking` for authenticated screens/API and preserve the intentional public `/book` experience.
- [x] Rewrite the fresh availability schema and every consumer to use `booking_availability`; no copy migration or old identity alias.
- [x] Update assignment source metadata, profiles, generated registries, translations and API consumers.
- [x] Remove the tasks implementation dependency and foreign SQL: meeting activities are scheduled, moved and canceled through the SDK `MeetingScheduler` capability, and owner busy intervals replace booking's `tasks` query.
- [ ] Remove remaining party and assignment implementation dependencies and foreign SQL.
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
- [x] Keep provider selection in telephony. Resolve contributions on each lookup, filter by captured owner availability, and retain provider-specific module restrictions. System scopes do not carry installation module lists; captured owner availability comes from the application registry.
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

## Portable configuration reference ownership

- [x] Define owner reference-key sources in the SDK, rejecting ambiguous providers for a kind. Normal consumers use enabled sources; full backups include stored references from disabled compiled owners.
- [x] Workflow provides pipeline, compound stage and lost-reason keys; tagging provides stable tag keys; tasks provides task-type keys. Their SQL stays in their owners.
- [x] Organization export reads contributed reference keys, removes workflow's implementation dependency and no longer queries tag/task-type tables, including formatted SQL.
- [ ] Adopt these sources in remaining configuration consumers and review disabled reference owners during export/restore.
- [ ] Manual acceptance: stable keys, duplicate normalized names, compound stage keys, custom stored keys and export/restore matching with owner combinations.

## Automation reference consumers

- [x] Owners publish package reference keys alongside stored keys. Sequences and segments provide their own metadata queries.
- [x] Automation package planning/export uses contributed tag, task-type, workflow, sequence and segment keys. Owners parse their package sections; automations retains trigger/action rewrites and conflict policy.
- [x] Remove automation's workflow/task/tagging implementation dependencies and foreign sequence/segment SQL, including the old missing-table fallback.
- [x] Share the generic in-memory typed condition evaluator through the SDK. Layout rules, repositories and execution remain in record layouts; automations has no peer implementation dependency.
- [ ] Manual acceptance: automation package references, compound stage rewrites, sequence enrollment, segment conditions, absent owners and invalid package sections.

## Shared condition evaluation

- [x] Move the generic typed in-memory filter evaluator and explicit evaluation context to the SDK. Existing comparison, accent/phone normalization, current-user and timezone/date semantics are unchanged.
- [x] Layouts and automation trigger conditions use the same utility without an implementation dependency.
- [ ] Manual acceptance: typed comparisons, empty values/groups, current user, relative dates/DST, layout rules and automation trigger conditions.

## Reminder task ownership and linked labels

- [x] Tasks owns due-reminder selection and conditional sent receipts through a bounded native SDK source; reminders keeps scheduling and delivery orchestration.
- [x] Share task notification metadata through a pure tasks contract and the linked-record response codec through the SDK.
- [x] Remove reminders' task implementation dependency and foreign task SQL. Standalone reminders remain available without tasks.
- [x] Load reminder linked titles in bounded scoped batches through registry metadata, respecting read permissions and hidden title fields instead of unscoped SQL title expressions.
- [ ] Manual acceptance: task reminders and receipts, disabled task source, retry/dedupe, standalone reminder CRUD, unreadable/hidden linked titles and notification failures.

## AI record context ownership

- [x] Define bounded native listing and related-record sources in the SDK, selected by record type and captured enabled contribution owner; ambiguous providers fail explicitly.
- [x] People/deals retain their list planners and batch projection; notes/tasks retain parent-link access, visibility and response policies. Owners supply the same response snapshots and masks to AI.
- [x] Remove all AI peer implementation dependencies. Single-record loads use the generic context handler guard instead of invoking handlers directly.
- [ ] Manual acceptance: AI context tools, related notes/private notes/tasks, user scopes and hidden fields, query filters, disabled owners and bounded source failures.

## Approval workflow references

- [x] Approval package export/planning uses owner-provided pipeline/stage keys and package metadata instead of workflow implementation types.
- [x] Workflow owns bounded stage-name lookup; approval validation and presentation consume the enabled catalog. Remove approval's foreign stage SQL and workflow crate dependency.
- [ ] Replace the remaining formulas implementation dependency.
- [ ] Manual acceptance: stage-change policies, package stage overrides, portable target resolution, disabled workflow and request stage labels.

## Bulk sequence enrollment

- [x] Publish enrollment origin metadata in a pure sequences contract and the native command/enrollment interface in the SDK.
- [x] Sequences retains actor permissions, recipient/mailbox checks, duplicate policy, persistence and jobs. Bulk actions calls the enabled service and rejects new enrollment jobs when it is absent.
- [x] Remove bulk actions' last peer implementation dependency. Pending executions resolve provider availability again.
- [ ] Manual acceptance: bulk enrollment permissions, duplicate recipients, inactive sequences, mailbox selection, disabled service and queued job retries.

## People organization snapshots

- [x] Organizations supplies scoped, masked list snapshots through the shared record context interface. People batches organization references through that source instead of joining organization tables.
- [x] People queries only its owned primary/secondary link rows and keeps owned link existence separate from the visibility of organization labels.
- [x] Share email engagement snapshot data in the SDK; each owner decodes its own persisted columns. People has no organization implementation dependency.
- [ ] Replace the remaining runtime organization prerequisite and foreign organization FKs with explicit soft-reference/deletion policies.
- [ ] Manual acceptance: primary/secondary company links, hidden name/domain fields, missing/unreadable/disabled organizations, link existence during merging and label ordering/pagination (now sorted in memory).

## Shared party identities and event identifiers

- [x] Publish people/organization identities and permission IDs through kernel-free capability contracts, with native public resource metadata in the SDK. CRM metadata delegates to those identifiers; codecs and storage remain owned.
- [x] Share booking/email event identifiers through pure contracts without changing logical event names or adding aliases.
- [x] Remove scoring and enrichment's remaining implementation dependencies, and sequences' booking dependency, when they only consumed public metadata.
- [ ] Review scoring's remaining runtime target prerequisites and party SQL; shared identifiers do not resolve those policies.
- [ ] Manual acceptance: public event envelopes/schema resources, scoring and sequence event subscriptions, owner registration, enrichment and capability combinations.

## Deal metadata and signing references

- [x] Publish deal identity in its pure contract and native public resource metadata in the SDK. Event consumers use shared lifecycle identifiers.
- [x] Remove metadata-only implementation dependencies from insights/messaging and sequences' deal event subscription.
- [x] Signing validates quote/deal association through the guarded owner record handler and its masked snapshot, with no direct quotes implementation. Missing, unreadable, disabled or hidden associations are rejected.
- [x] Signing, insights and messaging have no peer implementation dependencies; messaging's runtime people prerequisite and foreign SQL/FKs remain pending.
- [ ] Manual acceptance: signing with quotes and hidden associations, disabled owners, public event envelopes, insight widgets, messaging channel composition and sequence deal exits.

## Forecast probability ownership

- [x] Share probability source/group data through the pure deals contract and native bounded snapshot/read interfaces through the SDK.
- [x] Deals plans scoped queries and reads its own probability/credit data. Workflow supplies stage metadata, joined through bound snapshots rather than foreign table SQL.
- [x] Forecast keeps horizons, categories, weighted prediction and settings. Its store reads only its own settings and core organization currency; it has no peer implementation dependency or foreign SQL.
- [x] Shared credit readers expose owner-calculated monthly totals. Probability snapshots enforce field visibility, including amount/base amount/date and returned title/currency/owner/stage labels.
- [ ] Manual acceptance: forecast methods/categories/horizons, probability filters and pagination, split credit, scope and field masks, inactive stages and disabled owner catalogs.

## Pricing metadata and codecs

- [x] Publish catalog item identity/permission/tag/error metadata through a pure contract, with the native resource codec metadata in the SDK.
- [x] Pricing consumes shared currency normalization and a generic organization-scope authorization helper. Catalog/pricing calls retain their permission arguments and access rule.
- [x] Remove pricing's remaining peer implementation dependencies. This does not remove its foreign SQL/FKs or runtime catalog prerequisite; those remain pending.
- [ ] Manual acceptance: catalog/pricing permissions and scope, bundle errors, currency parsing, public schema tags and optional capabilities.

## Pricing currency and segment owners

- [x] Read enabled non-base currencies through the enabled money rates interface; base currency remains core organization metadata. Money retains its currency-table SQL.
- [x] Publish saved-filter identity and a native readable target interface. Segments retains lookup, parent read authorization and enabled-parent checks.
- [x] Pricing no longer queries money currency or segment tables directly. Disabled owners do not validate stale foreign metadata through raw SQL; staged currency declarations are considered only when the money provider is enabled.
- [ ] Manual acceptance: base/foreign currency lists, disabled money, segment pricing, unreadable or disabled segment parents and saved filter deletion.

## Scoped pricing reference identities

- [x] Share bounded scoped identity selection through the SDK's generic record query utility; label hydration uses the same selection path.
- [x] Pricing validates company identities through scoped registry definitions; catalog identity validation delegates to its owner, including the existing settings-administrator access path. Neither formats foreign table names into SQL.
- [x] Remove the two formatted cross-owner existence queries that the literal SQL ratchet did not identify.
- [ ] Manual acceptance: pricing company/item selection, own/team/organization scopes, missing read permissions, hidden labels, deleted references and disabled owners.

## Pricing catalog data ownership

- [x] Separate owned price-list entries from catalog presentation snapshots. Pricing reads only its own item rows; bounded catalog sources provide visible labels, codes, base prices and currencies.
- [x] Catalog items owns portable code lookup through a settings-authorized native interface. Configuration plans retain live/null-code distinctions and the existing active-code lookup semantics.
- [x] Bundle component checks and price tier resolution validate target identities through the enabled catalog reference service, enforcing organization read scope or settings administration instead of joining catalog tables.
- [x] Remove all remaining pricing foreign SQL, including product code lookups and bundle joins. Catalog snapshot reads enforce organization scope or the existing settings-administrator path.
- [ ] Replace pricing's foreign FKs and runtime catalog prerequisite with explicit removal policies.
- [ ] Manual acceptance: price tiers, labels/field masks, bundle nesting, live/deleted products, code-less products, configuration round trips, replacement of orphaned owned rows, ordering and disabled sources.

## Scoped task link titles

- [x] Remove dynamic foreign-table title subqueries from task and timeline reads. Task storage selects only owned links and core user metadata.
- [x] Hydrate primary and secondary linked titles through bounded scoped record labels, including target read permission, field visibility and enabled native owners.
- [x] Keep unreadable linked titles empty without changing stored relationships or task write semantics.
- [ ] Manual acceptance: task lists/details and timeline links, hidden title fields, own/team scopes, deleted targets and disabled owners.
- [ ] Remaining task calendar persistence must still move behind owner interfaces.

## Task inbox deal snapshots

- [x] Select task-to-deal links exclusively from task-owned tables, then request bounded scoped deal listings from the enabled owner.
- [x] Remove the remaining literal task foreign SQL. Owner snapshots apply record constraints and field masks before inbox selection and sorting.
- [x] Preserve primary-link precedence and highest readable amount for secondary links, with deterministic identity ties; disabled deals yield no projection.
- [ ] Manual acceptance: inbox deal sorting, archived/deleted deals, primary/secondary links, hidden amounts/currencies/titles and disabled deal sources.

## Marketplace installed package catalog

- [x] Configuration packages exposes settings-authorized installed package/version summaries through its enabled catalog service.
- [x] Marketplace no longer instantiates or imports the configuration version repository. A missing catalog provider yields no installed template versions while theme and module discovery remain available.
- [x] Publish pure package identity/version metadata and native version commands, snapshots and transport codecs. Marketplace invokes enabled comparison/application services; configuration retains planning, transactions, conflict policy, storage and audit.
- [x] Remove marketplace's last peer implementation dependency. Public OpenAPI remains semantically unchanged after codec extraction.
- [ ] Manual acceptance: catalog installed/update flags, disabled configuration packages, settings authorization and theme discovery.

## Approval formula predicates

- [x] Expose native formula predicate validation/evaluation through an enabled SDK service. Formula workspace compilation, source reads, dependency ordering and evaluation stay with formulas.
- [x] Approvals retains its condition definition and existing elevated policy scope, and maps owner diagnostics to its own validation field.
- [x] Remove approvals' final peer implementation dependency; missing formula providers fail explicitly for formula conditions instead of executing disabled implementation code.
- [ ] Manual acceptance: boolean conditions, invalid expressions, aggregate/custom dependencies, policy evaluation scope and disabled formulas.

## Configuration formula planning

- [x] Formula owner validates projected field expressions, result types and dependency cycles against the provided definition reader and all projected overrides.
- [x] Configuration maps owner diagnostic snapshots to package paths and requests owner-produced recalculation job metadata while retaining its own apply transaction.
- [x] Remove configuration packages' final peer implementation dependency. Packages without formula projections work without a formula provider; formula planning/application requires the enabled owner.
- [ ] Manual acceptance: projected interdependent fields, cycles and type errors, recalculation enqueue, transactional rollback and disabled formula owner.

## Scoring primary organization ownership

- [x] People exposes bounded system-authorized primary organization snapshots, including explicit live/deleted selection and keyset-paged live people by organization.
- [x] Scoring selects and rolls up only its own signals; organization contributions use owner snapshots with bounded signal queries. Deleted contacts remain excluded from rollups while lifecycle recalculation can identify their former organization.
- [x] Remove all scoring foreign SQL and both hard party prerequisites. A trait-based registry transform supplies score fields to declared scoring subjects; missing optional people sources contribute no rollup.
- [x] Declare every scoring-owned table in its manifest.
- [ ] Manual acceptance: direct/people-derived organization scores, signal decay, deleted/restored/reassigned people, large paged organizations, disabled owners and custom scoring subjects.
- [ ] Audit generic scoring query availability and policy constraints alongside managed entity availability.

## Email party capture ownership

- [x] Email creates people and organizations through enabled, authorized generic record handlers, preserving owner normalization, validation, audit and event execution.
- [x] Party capture defaults are optional owner contributions with explicit collision rejection. Leads supplies the existing active lead-stage default for captured people; email no longer imports lifecycle implementation or constants.
- [x] Remove email's people, organizations and leads implementation dependencies. Its remaining SSO dependency, foreign directory SQL and runtime party prerequisites remain pending.
- [ ] Manual acceptance: reciprocal email capture, organization creation, lifecycle defaults, disabled leads/party owners, actor scopes and field-write permissions.

## Scoped email party matching

- [x] Match participant/recipient email addresses through paged owner record listings, preserving oldest-first selection and the separate archived policies for synchronization and outbound messages.
- [x] Resolve capture organizations by domain through owner listings, preferring active over archived records. Read permission and lookup-field visibility are required before matching.
- [x] Remove email's direct person address and organization domain SQL; unrelated owned email/core-user reads remain in email storage.
- [ ] Manual acceptance: duplicate email addresses, archived matches, hidden email/domain fields, mailbox actor scopes, missing owner sources and domain capture ordering.
- [x] Related organization/open-deal linking uses optional party association sources. People and deals select only their own links; bounded owner listings validate target availability, read scopes and archived/deleted state before email stores references.
- [ ] Manual acceptance: primary/secondary company links, open deal contacts, hidden association fields and disabled association/target sources.

## Email engagement owner projections

- [x] Email aggregates delivered message engagement exclusively from email-owned tables through a transactional snapshot reader.
- [x] People and organizations lock and update their own projection rows in the same transaction as the snapshot read, retaining unchanged record versions and serialized refreshes.
- [x] Projection sinks explicitly opt into maintenance while compiled but disabled; event processing uses compiled sinks and rejects ambiguous target ownership instead of dropping refreshes.
- [x] Remove email's formatted cross-owner UPDATE, which the literal SQL ratchet did not identify. Email has no remaining party SQL or hard party prerequisites; unavailable party capture is skipped, and unavailable organization capture does not prevent person capture.
- [ ] Manual acceptance: received/sent/link/unlink refreshes, distinct message counts, latest sender, empty snapshots, concurrent refreshes, unchanged versions and disabled projection owners.
- [ ] Email still depends on SSO OAuth implementation; that extraction remains pending.

## Shared lead conversion extension points

- [x] Publish leads identity and canonical native qualification/conversion hook metadata through a kernel-free contract.
- [x] Share lead snapshots, conversion commands/results, pending commits and the transactional converter interface through the native SDK. Lead state rules, default conversion mapping and actual writes stay in leads.
- [x] Lead Pro decorates the shared converter and consumes shared hook types; remove its leads/deals implementation dependencies and use owner resource metadata.
- [x] Lead Pro reads authorized people through owner handlers/listings, respecting returned field masks. Owner notifications read the owner's scoped record snapshot.
- [x] Heat/cooldown update only Lead Pro tables; generic target version locks preserve live-parent checks and serialize touches against bounded cooldown batches. Remove all three remaining foreign people queries and the final peer implementation dependency.
- [ ] Manual acceptance: hot board masks/scopes, touch idempotency, cooldown pagination, concurrent touches/deletion, owner notifications and disabled people.
- [ ] Manual acceptance: qualification veto, conversion enrichment, decorator touch insertion in the same transaction, rollback/pending commits and optional converter owners.

## OAuth state schema ownership

- [x] SSO stores nonce/PKCE state in its own table with a bound provider key and provider/hash uniqueness. Remove dynamic provider-table SQL while retaining expiry and transactional single-use consumption.
- [x] Remove Google/Microsoft OAuth state table creation from fresh provider schemas. Login, calendar and email use provider-partitioned state; calendar purpose metadata belongs to tasks.
- [x] Rewrite schemas directly without adoption/backfill or compatibility handling.
- [ ] Shared OAuth interfaces, provider-owned calendar persistence and removal of remaining peer implementations are pending.
- [ ] Manual acceptance: provider isolation, expired/replayed state, concurrent callbacks, login/calendar/mail purpose dispatch and fresh installation schemas.

## Shared OAuth protocol interface

- [x] Publish the SSO identity/protocol capability descriptor through a kernel-free contract and native protocol/data interfaces through the SDK.
- [x] Share only OAuth data, token/claim codecs, PKCE and URL construction. SSO retains the configured HTTP transport, token requests, error responses and cached endpoint discovery.
- [x] Email, tasks, Google and Microsoft request protocol operations through an enabled service rather than importing SSO protocol execution code.
- [ ] Extract identity flow/state interfaces and generic integration helpers to remove the remaining SSO implementation dependencies.
- [ ] Manual acceptance: authorization URLs, PKCE, token exchange/refresh, provider errors, claims, discovery caching and absent/disabled protocol providers.

## OAuth state capability interface

- [x] Share state request/result data and an enabled native state-store interface. Email and task calendar connections create states through the owner service.
- [x] Keep random state issuance, hashes, expiration, row locks and single-use deletion in SSO; no state SQL or persistence implementation moves to the SDK.
- [ ] Manual acceptance: login/calendar/mail state creation, unavailable state providers, provider isolation, expiry and replay rejection.

## Shared identity flow interfaces

- [x] Publish native identity provider/query/status models and enabled login/status/credential/exchange interfaces. SSO retains account discovery, allowlist enforcement, redirect validation, rate decisions, token exchange checks, session creation and callbacks.
- [x] Share generic core integration helpers for settings, secrets, user contexts and rate limiting; shared error/response codecs and configuration metadata contain no plugin table SQL.
- [x] Keep the SSO login settings policy model in its owner. Microsoft verifies its own preferred-username/domain fallback using shared parsing utilities.
- [x] Remove SSO implementation dependencies from email, tasks, Google and Microsoft. Email and tasks now have no peer implementation dependencies; vendor providers still consume email/task implementations pending extraction.
- [ ] Manual acceptance: account discovery, login domains, required SSO, provider-specific claims, state/purpose callbacks, session cookies, credential lookup, revoked access and disabled flow providers.

## Task calendar persistence ownership

- [x] Tasks owns provider-partitioned calendar connections and remote links in its own fresh schema; Google and Microsoft contribute no SQL schemas.
- [x] Replace all provider-formatted calendar table SQL with fixed task-owned tables and bound provider keys. Preserve per-provider connection, remote-event and task-link uniqueness.
- [x] Remove the provider-table dependency missed by the literal SQL ratchet without introducing adoption, copy or compatibility migrations.
- [ ] Extract shared calendar provider interfaces and owner job orchestration; review provider removal/secret cleanup through owner maintenance policies.
- [ ] Manual acceptance: fresh schema, provider isolation, connection reconnect/revoke, cursor/count updates, link upsert/delete, concurrent sync and provider export/removal behavior.

## Shared remote calendar provider data

- [x] Publish native calendar adapter interfaces, event/change snapshots, connection DTOs and configuration metadata through the task SDK surface.
- [x] Share only timestamp/URL codecs alongside those models. Task synchronization, network request handling, connection/token policy and persistence remain with existing owners.
- [x] Google and Microsoft contribute calendar adapters through a declared capability; they use shared provider data instead of task implementation models.
- [x] Expose remote calendar connection/status/sync/availability execution through enabled task services; keep HTTP response handling, token policy, synchronization and persistence in tasks.
- [x] Tasks owns one scheduler and sync/push handlers, with provider identity in payloads and fresh enabled-provider resolution per execution. Vendor adapters register only their capability; ambiguous keys fail explicitly.
- [x] Remove Google/Microsoft task implementation dependencies. Optional task absence produces unavailable connection actions, empty availability and a non-connectable status; calendar callbacks reject wrong state purposes.
- [ ] Manual acceptance: both providers, job dispatch/deduplication/retries, disabled owners/adapters before queued work, new provider activation, callback purposes and event fanout failures.
- [ ] Manual acceptance: calendar adapter payloads, settings metadata, timestamp/segment codecs, event windows and public response schemas.

## Shared mailbox provider models

- [x] Publish mailbox provider interfaces, account/credential callbacks, message/attachment/sync snapshots and provider error codecs through the native email SDK.
- [x] Keep provider selection, mailbox storage, vault writes and best-effort refreshed-credential persistence policy in email. Shared persistence handles only forward to the owner's callback.
- [x] Google/Microsoft consume shared provider models without importing email implementation types.
- [x] Share MIME/base64 wire codecs, OAuth provider metadata and generic callback adapters. Keep token expiry/refresh/revocation, persistence, recovery decisions, HTTP response handling and mailbox connection execution in email.
- [x] Resolve enabled OAuth contributors by their mailbox provider identity; ambiguous sources fail. Owner services decide whether a failed operation can receive one refreshed-token retry.
- [x] Remove Google/Microsoft's last email implementation dependencies. Both vendor plugins now depend only on shared contracts/SDK and the kernel.
- [x] Reject wrong-purpose mailbox OAuth callback states; absent mailbox execution fails explicitly.
- [ ] Manual acceptance: MIME round trips, initial/expired/rejected/revoked tokens, refresh persistence failures, single retry, callbacks and disabled/ambiguous providers.
- [ ] Manual acceptance: IMAP/vendor providers, callback persistence, redacted debug output, provider errors and message/sync codecs.

## Scoped mailbox and messaging execution

- [x] Native mailbox and messaging providers receive the actual caller context. Vendor adapters no longer reconstruct global application state or replace caller scope with a system scope.
- [x] Messaging delivery/polling passes the selected acting-member scope; IMAP and WhatsApp retain their existing transport behavior.
- [x] Cached mailbox registrations retain owner identity and check current owner/module availability at lookup. Reject ambiguous provider keys instead of silently replacing implementations; preserve explicit member module restrictions and system scopes without module claims.
- [x] Remove obsolete global provider context helpers from the SDK surface.
- [ ] Manual acceptance: native/vendor mailbox operations, scheduled/user delivery, messaging actor selection, disabled cached providers, duplicate providers and restricted/system module scopes.

## Bounded mail provider responses

- [x] Enforce the existing 40 MiB response limit while reading actual provider chunks, including responses with absent or misleading content lengths. Keep transport execution in email.
- [x] Validate formatting and strict email Clippy without running tests.
- [ ] Manual acceptance: exact-limit bodies, oversized/chunked bodies, misleading lengths and interrupted streams.

## Import source and reference interface

- [x] Share parsed source snapshots and the pure header normalization codec through contracts/SDK. Keep CSV/spreadsheet parsing, bounds and reference lookup execution in imports.
- [x] The migration assistant requests source parsing and scoped record/member lookups through an enabled owner service; unavailable native entity owners return no lookup.
- [x] Formatting and strict imports/migration-assistant Clippy pass without tests.
- [ ] Manual acceptance: CSV encodings/delimiters, spreadsheet dates, parser limits, scoped label aliases, inactive members and unavailable providers.

## Import draft owner service

- [x] Publish import draft commands and response data through the SDK; native DTO conversions remain in imports.
- [x] The migration assistant creates/configures drafts through an enabled owner service. Imports retains permissions, supported/default duplicate-key selection, draft ownership, mapping validation, storage and persistence.
- [x] Remove the assistant's imports implementation dependency and make imports an optional capability. Full lint passes and generated OpenAPI is unchanged.
- [ ] Manual acceptance: recognized/suggested mappings, unsupported duplicate keys, actor ownership, unavailable imports and file/storage failures.

## Task and note creation interfaces

- [x] Publish task/note creation commands and enabled writer interfaces through the SDK; share data only. Task field-access policy, reference validation, hooks, transactions, events and audit remain with their owners.
- [x] Migration history and demo seeding pass their real actor context and audit IP to owner writers. The migration assistant now has no peer implementation dependencies; demo no longer imports task/note implementations.
- [x] Full lint passes; lock the native peer-dependent plugin count from nine to eight without increasing other ratchets.
- [ ] Manual acceptance: migrated task links/types/completion/ownership, note visibility/mentions, audit IP, unavailable writers and demo bookkeeping.

## Task type catalog interface

- [x] Share task-type snapshots and an enabled native catalog interface. Tasks retains reader permissions, active filtering, ordering, storage and pagination.
- [x] Sequences creates tasks and reads types through owner services, removing its tasks implementation dependency; booking reads meeting types through the same catalog.
- [x] Formatting and strict task/sequence/booking Clippy pass without tests.
- [ ] Manual acceptance: task type permissions, active/inactive options, sequence fallback selection, booking meeting lookup and disabled task capabilities.

## Forms capability interfaces

- [x] Forms consumes shared people metadata and lead conversion hooks instead of peer implementation crates. Native entity actions retain owner handlers.
- [x] Workflow owns scoped pipeline views, reader authorization, team visibility and stage snapshots. Forms only maps the returned choices.
- [x] Assignment owns active rule options, rule usability and idempotent owner selection. Shared selector resolution rejects ambiguity; forms preserves fixed-owner fallback and warnings on unavailable/failed distribution.
- [x] Remove all six forms peer implementation dependencies and its hard people prerequisite. Full lint passes; OpenAPI is unchanged. Native peer-dependent plugins fall from eight to seven and layer violations from five to three.
- [ ] Manual acceptance: optional owners, form options, restricted workflow teams, lead hooks, active rule choices, idempotent distribution and fixed-owner fallback/warnings.

## Prospective formula evaluation interface

- [x] Formulas owns boolean compilation, dependency resolution, proposed-record evaluation and preview execution behind an enabled native interface. Share commands, diagnostics and response snapshots only; no compiler, AST, evaluator or formula SQL moves to the SDK.
- [x] Validation rules retains rule CRUD, limits, timing, field/message selection, invalid-rule warning/skip policy and ordered violations. Custom field values are read through the existing callback using the caller's transaction; scoped system values and aggregates retain their original sources.
- [x] Unavailable formula execution fails explicitly when active rules need it; records without applicable rules do not require the capability.
- [x] Remove the validation-rules formula implementation dependency. Full lint passes, OpenAPI is unchanged and native peer-dependent plugins fall from seven to six.
- [ ] Manual acceptance: create/update timing, boolean/type diagnostics, formula dependencies/aggregates, custom values in the write transaction, invalid saved expressions, ordered field errors, preview scopes and disabled formulas.

## Shared mail content codecs

- [x] Share the existing address normalization/length codec and objection event identity through the email contract; share normalized address hashing and generic HTML sanitization/text codecs through the SDK.
- [x] Keep tracking URL selection, link rewriting and pixel insertion in email. No transport, template policy or plugin SQL moves to shared utilities.
- [x] Booking no longer imports the email implementation; sequences consumes shared content codecs. Formatting and strict email/booking/sequence Clippy pass without tests.
- [ ] Manual acceptance: address validation/hashing, dangerous HTML removal, HTML text conversion, booking addresses and sequence content.

## Sequence mail execution interfaces

- [x] Extend the enabled email sender interface with owner-executed sending and template preview; publish preview commands/results and sent-message identity snapshots. Email retains template resolution, mailbox/transport policy, permissions, delivery and persistence.
- [x] Sequences consumes shared email commands and people event metadata. Remove its last email/people implementation dependencies and use optional owner capabilities with explicit unavailable errors.
- [x] Full lint passes and native peer-dependent plugins fall from six to five. Foreign SQL and FK cleanup for sequences remains pending.
- [ ] Manual acceptance: sequence sends, threaded replies, template variables, manual drafts, task-only steps, sender scopes and unavailable email/people owners.

## Sequence mail data ownership

- [x] Email executes organization-template key lookups, tracking-objection lookups and owner-filtered mailbox selection through a native automation reader. All five foreign SQL references are removed from sequences; transaction-local template lookup still uses the caller's transaction.
- [x] Missing email yields no template choices/mailboxes; required consent checks fail explicitly. Configuration export/planning refuses unresolved stored template identities instead of silently dropping references.
- [x] Formatting and strict email/sequence Clippy pass; foreign SQL references fall from 43 to 38 without raising other ratchets.
- [ ] Manual acceptance: template mapping/export/apply, unavailable catalogs, consent objections, mailbox ownership/status ordering and task-only sequences without email.

## Scoped sequence person snapshots

- [x] Remove people joins/raw reads from enrollment display, consent-event matching and sequence target resolution. Read bounded, deduplicated owner snapshots and preserve owner field masking.
- [x] Enrollment responses hydrate only readable person names/emails; inaccessible labels remain empty. Worker target resolution uses the actual enroller context and treats unavailable/deleted targets as unavailable.
- [x] Keep enrollment selection/counting/locking in sequences. Strict sequence Clippy passes and foreign SQL references fall from 38 to 35.
- [ ] Manual acceptance: enrollment displays/counts, masked email/name fields, lost target permissions, deleted/disabled people, objection events and scheduled actor scopes.

## Sequence deal lifecycle ownership

- [x] Deals owns primary/secondary person lookup for lifecycle consumers through a system-only native interface; sequences no longer reads deal tables.
- [x] Scoped deals/sequence compilation passes. Foreign SQL and cleanup policies remain tracked separately.
- [ ] Manual acceptance: won/lost exits, secondary people, unavailable owners and rejection of non-system maintenance.

## Booking party and assignment interfaces

- [x] Booking consumes scoped person snapshots and transactional person creation through SDK interfaces, preserving savepoint fallback, custom answers, audit IP and post-commit publication.
- [x] Assignment owns host pool resolution, strategy/workload selection, locked counters and pick recording. Booking receives one coherent rule/pool snapshot; disabled rules cannot be selected.
- [x] Replace the foreign rule-label join with bounded owner label reads. Booking has no peer implementation dependencies or foreign SQL; scoped compilation passes without warnings.
- [ ] Manual acceptance: new/existing people, mapped-answer fallback, meeting transactions, weighted/least-loaded selection, away intervals, rule disablement and labels.

## Renewal workflow contracts

- [x] Renewals reads pipeline/stage catalogs through the enabled workflow SDK reader. Shared portable package models and key codecs contain no SQL or workflow execution.
- [x] Remove the workflow implementation dependency; scoped workflow/renewal compilation passes without warnings.
- [ ] Manual acceptance: renewal pipeline/stage validation, portable keys, package planning and unavailable workflow.

## Renewal deal and revenue interfaces

- [x] Renewals consumes redacted owner deal snapshots and an owner creation interface; deals retains validation, workflow hooks, revenue initialization and persistence. Preserve the unassigned-owner retry.
- [x] Revenue aggregation, field visibility, exchange-rate conversion and partial-result policy remain in deals_revenue behind a summary interface; shared values contain only totals and formatting codecs.
- [x] Remove remaining renewal peer implementation dependencies; scoped deals/revenue/renewal compilation passes without warnings. Foreign SQL/FK cleanup remains pending.
- [ ] Manual acceptance: original field visibility, owner fallback, renewal amounts/dates/links, recurring items, currency conversion, partial totals and unavailable owners.

## Renewal candidate ownership

- [x] Deals owns scoped, ordered candidate pages and existing-renewal lookup, including the existing deleted-renewal behavior.
- [x] Renewals filters only its own completion receipts and continues through candidate pages until the original 500-item limit; completed rows cannot starve later candidates.
- [x] Scoped deals/renewal compilation passes; two foreign SQL references are removed.
- [ ] Manual acceptance: settings-author scope, date boundaries, pagination after completed rows, deleted renewals and concurrent renewal discovery.

## Renewal widget ownership

- [x] Deals executes bounded widget queries, record constraints and field masking. Workflow supplies stage labels through its owner catalog; no foreign SQL or global pool reconstruction remains in renewals.
- [x] Preserve item order and aggregate totals; hidden fields are omitted/empty and unavailable stage labels remain empty. Scoped deals/renewal compilation passes.
- [ ] Manual acceptance: widget ordering/limits, hidden title/amount/currency/owner/stage fields, record constraints and disabled owners.

## Telephony task transactions

- [x] Tasks owns completed-task preparation and transaction insertion through opaque prepared-task handles. Each handle exposes only its identity, linked reference, audit IP and its own post-write response; owner availability is rechecked.
- [x] Telephony uses owner-authorized task reads and shared response models/codecs; final responses retain field redaction. No task engine, query or storage implementation moves to the SDK.
- [x] Remove telephony's tasks implementation dependency. Scoped tasks/telephony compilation passes without warnings; SQL/FK cleanup remains pending.
- [ ] Manual acceptance: call logging/update/provider events, session calls, task hooks, rollback atomicity, audit IP, recurrence DTOs, scopes and disabled tasks.

## Telephony data ownership

- [x] Call details query only owned tables. Tasks owns creation-type lookup and bounded, scoped report subjects; telephony aggregates owned calls. Preserve completion/creation date semantics, owner filtering and missing owner masks.
- [x] Session person names use scoped record labels. Administrators can manage owned outcomes without tasks; task-only create permission remains sufficient to resolve creation types.
- [x] Remove all six foreign SQL references; scoped tasks/telephony compilation passes without warnings.
- [ ] Manual acceptance: report dates/scopes/pagination/totals, soft-deleted provider calls, create-only actors, queue label masks and disabled owners.

## Renewal notification lookup

- [x] Move the remaining notification title/owner lookup into deals through its enabled renewal reader. Renewals now has no literal foreign SQL; scoped compilation passes.
- [ ] Manual acceptance: notification recipients, deleted deals and disabled deal readers.

## Duplicate lifecycle and trash interfaces

- [x] Duplicates consumes the shared trash exclusion point and delegates transactional lead stage reads/history to leads; remove both implementation dependencies. Use the generic domain normalization codec directly.
- [x] Scoped trash/leads/duplicate compilation passes without warnings. Party merge repositories and formatted foreign rewrites remain pending.
- [ ] Manual acceptance: lifecycle history, optional leads/trash and merged-record exclusion.

## Outstanding scope

The remaining identity changes, shared capability interfaces, owner-only SQL, deletion policies and per-plugin manual acceptance matrices are still pending. Existing architectural debt is not considered resolved by renaming packages.
