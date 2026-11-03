# Optional platform capabilities

P4 moves marketplace, demo policy, dashboard, integrations, validation rules, indexed search, notifications and push, automations, webhook delivery, configuration packages, object administration, saved views, record layouts and formulas out of the kernel.

Each owner registers its HTTP routes, workers, hooks and contributions. The kernel retains dynamic entity storage, generic record/relation routes, field metadata and permissions, query compilation, transactional events/jobs, identity, mail/file/secret resources, safe outbound HTTP and installation services. `aphel-contract-automation` contains the shared trigger/action/gateway definitions and helpers; automation execution and operation adapters belong to the plugin.

## Composition

The generated `plugins` Cargo feature is enabled by default. Run `cargo check -p aphel --no-default-features` from `back/` to compile the production binary with only `KernelModule`. No alternate server, test binary or database instance is needed. `xtask modules sync` owns the dependency features and conditional registry entries.

The frontend trusts the server's module inventory, including for formerly required modules. Missing plugins contribute no active routes, navigation, slots or lifecycle work. Record layouts fall back to the full field list; saved-view controls disappear; disabled queries do not hold forms/listings in a loading state. Search and push make no requests when their plugin is absent.

Installer profiles, their answer validation and configuration templates live in `blueprints/`. The host accepts an opaque installation input through `InstallerBlueprintSource`; a build without a blueprint skips business/profile selection.

## Extension points

- `HttpRequestPolicy` implements request restrictions such as demo mode. `MailDeliveryPolicy` can replace delivery for a demo installation.
- `FieldDefinitionValidator` and `ConditionalRulesFactory` provide formula validation and layout conditions without direct kernel dependencies on either implementation.
- `session.revoked` runs inside the session revocation transaction. Notifications contributes push-subscription cleanup for single-session, all-session, other-session and member revocation paths.
- `OrganizationConfigExtension` supplies optional settings defaults/validation. Marketplace and webhooks own their configuration projections; the core preserves unrelated settings through flattened JSON extensions.
- `ConfigSectionFactory`, `SectionFragmentLayout` and `NestedFragmentLayout` describe portable configuration and fragment reconciliation. Object administration, layouts, views, validation rules and native domains register their own sections. Fragment processing has no fixed section-name dispatch.
- `FilterSourceResolver` expands saved predicates; `ReferenceSetField`/`ReferenceSetStorage` describe contributed multi-reference filtering and aggregate labels. Saved-filter predicates use the `in_saved_filter` JSON operator.
- `CatalogExtension` adds owner projections, including notification kinds, to the host catalog.

## Fresh-installation schema

The project has no production deployment or persisted data to preserve. The user explicitly approved rewriting schemas for fresh installations. Migration adoption, legacy entity aliases, checksum-replacement exceptions and automatic module activation from historical core migrations have been removed. Normal schema creation and migration checksums remain.

SMTP settings are core `mail` configuration. Update `mail_password` to store a secret in the vault; omission preserves it and `null` clears it. Responses expose only `mail_secret_configured`. `SMTP_HOST`, `SMTP_FROM`, `SMTP_PORT`, `SMTP_SECURITY`, `SMTP_USERNAME` and `SMTP_PASSWORD` remain infrastructure fallback. Setup and settings use the same core mail form; there is no SMTP integration compatibility transport. Webhook signing and delivery headers are constructed by capability owners; the core HTTP transport only enforces network/header safety.

## Architecture accounting and remaining phases

Extracted files now count as plugin code rather than kernel code. P4 therefore rebases the architecture inventory for the new ownership boundary, including newly visible dependency edges, SQL references and frontend imports. This is recorded explicitly rather than interpreted as a decrease in coupling. The ownership rebase changes direct-plugin dependency crates from 41 to 57, cross-plugin foreign keys from 34 to 36 and cross-plugin SQL findings from 156 to 159; these are exposed migration debt for P5. Kernel public modules fall from 462 to 325 and shell domain-word occurrences from 124 to 99. Kernel SQL on plugin tables, foreign-table alterations, invalid plugin manifests and kernel domain-word/permission/record-type counters must remain zero.

P5 still dissolves native plugin dependencies and replaces the transitional CRM contract, including base-currency ownership and source-driven dashboard/webhook projections. P7/P8 own the remaining web composition and blueprint-selection changes. These phases are not replaced by capability extraction.

Validation follows D4: compilation, strict Clippy, formatting, static architecture/module checks, TypeScript/lint and production OpenAPI/catalog generation. No tests, database probes, smoke runs or scratch instances are written or run. Schema changes are reviewed statically; database execution is not claimed as evidence.
