# Optional platform capabilities

Marketplace, demo policy, dashboard, integrations, validation rules, indexed search, notifications and push, automations, webhook delivery, configuration packages, object administration, saved views, record layouts and formulas are optional capabilities owned by their own modules, not by the kernel.

Each owner registers its HTTP routes, workers, hooks and contributions. The kernel retains dynamic entity storage, generic record/relation routes, field metadata and permissions, query compilation, transactional events/jobs, identity, mail/file/secret resources, safe outbound HTTP and installation services. `aphel-contract-automation` contains the shared trigger/action/gateway definitions and helpers; automation execution and operation adapters belong to the plugin.

## Composition

The generated `plugins` Cargo feature is enabled by default. Run `cargo check -p aphel --no-default-features` from `back/` to compile the production binary with only `KernelModule`. No alternate server, test binary or database instance is needed. `xtask modules sync` owns the dependency features and conditional registry entries.

The frontend trusts the server's module inventory. Missing plugins contribute no active routes, navigation, slots or lifecycle work. Record layouts fall back to the full field list; saved-view controls disappear; disabled queries do not hold forms/listings in a loading state. Search and push make no requests when their plugin is absent.

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

There is no production deployment or persisted data to preserve, so schemas are written for fresh installations: no migration adoption, no legacy entity aliases and no checksum-replacement exceptions. Normal schema creation and migration checksums apply.

SMTP settings are core `mail` configuration. Update `mail_password` to store a secret in the vault; omission preserves it and `null` clears it. Responses expose only `mail_secret_configured`. `SMTP_HOST`, `SMTP_FROM`, `SMTP_PORT`, `SMTP_SECURITY`, `SMTP_USERNAME` and `SMTP_PASSWORD` remain infrastructure fallback. Setup and settings use the same core mail form; there is no SMTP integration compatibility transport. Webhook signing and delivery headers are constructed by capability owners; the core HTTP transport only enforces network/header safety.

## Architecture accounting

Files extracted from the kernel count as plugin code. The architecture ratchet (`cargo run -p xtask -- arch check`) compares the inventory with `back/xtask/arch-baseline.json`; kernel SQL on plugin tables, foreign-table alterations, invalid plugin manifests and kernel domain-word, permission and record-type counters must stay at zero.

## Validation

Day-to-day validation is compilation, strict Clippy, formatting, the static architecture and module checks, TypeScript and lint, and production OpenAPI and catalog generation. The only automated check beyond compilation is the phase-gate probe (`make probe`). Schema changes are reviewed statically.
