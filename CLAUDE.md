# Aphel

Aphel is an open source platform for composing business systems from modules and plugins. The core knows no business domain: a blueprint (`blueprints/<key>`) picks the modules, configuration and navigation of a system, and a blank system is valid. Today's product is the `crm` blueprint and its variants (freelancer, agency, real estate, consultancy, SaaS, B2B sales, retail); fields, workflow, automations and plugins adapt each installation without code changes.

It ships as one Rust binary with the React app embedded. Each company (the "organization") has its own MySQL database. A process serves one company (`DATABASE_URL`) or many (`CONTROL_DATABASE_URL`, routed by host, with an operator console). What goes into the binary is chosen in `back/aphel.modules.toml`.

There is no production installation yet: schemas are rewritten for fresh installs instead of migrated (see Schema policy).

## Repository map

| Path | Contents |
| --- | --- |
| `back/kernel` | Crate `aphel-kernel`: HTTP/security, identity and access, entity/query/record engine, jobs, installer, settings, module system, multi-tenancy (`src/tenancy`). Migrations in `migrations/` (tenant databases) and `control_migrations/` (control database) |
| `back/modules/<key>` | Crates `aphel-mod-<key>`: optional capabilities and the CRM domain (`src/` + `migrations/`) |
| `back/contracts/<key>` | Contract crates (`aphel-contract-<key>`): traits, keys and DTOs shared between modules; `contracts/crm` holds `CrmPermissions`, `CrmRecordTypes`, field helpers and `keys` |
| `back/contract` | `aphel-contract`: contract primitives (`Contract`, `OperationKey`, `EventKey`, `HookKey`, `ExtensionPoint`, `ReferenceKind`, `Catalog`) |
| `back/sdk` | `aphel-sdk`: the API native modules use (re-exports of the kernel under `aphel_sdk::platform::*`, `Ctx`, `AppError`, `Runtime`) |
| `back/blueprint`, `blueprints/` | Blueprint crate and the blueprints (`<key>/blueprint.toml`, `installer.rs`) |
| `back/plugins/lead_pro` | Example of a compiled module kept out of the default build |
| `back/app` | Crate `aphel`: the binary, its CLI (`src/cli`) and the generated `src/modules.rs` |
| `back/xtask` | `modules sync`, `themes sync`, `arch check`/`arch baseline` |
| `back/openapi.json`, `back/console-openapi.json` | Generated API documents (tenant API and console API) |
| `web/` | React app: `src/app` (shell), `src/kernel` (web module system), `src/modules/<key>` (web modules), `src/features` (platform screens), `src/components` (UI), `src/sdk` (public web SDK), `src/console` (operator console), `src/api` (client and generated types) |
| `web/docs` | Public documentation site (Astro); pages in `src/content/docs/{start,using,admin,plugins,reference}`, served at `/_aphel/docs` |
| `sdk/rust/aphel-plugin-sdk`, `sdk/js/aphel-plugin-sdk`, `sdk/cli`, `sdk/js/aphel-ui` | SDKs and CLI for runtime plugins (own workspaces) |
| `docs/` | Only files the code or scripts rely on: generated `modules/hooks.md`, `plugins/host-catalog.md`, `ui/tokens.md`, backup scripts |
| `docker/`, `docker-compose*.yml` | Production image and local services |

Runtime plugins built for real customers live outside this repository (`../plugins/<name>`, own git repositories).

## Global decisions (do not reopen)

| Area | Decision |
| --- | --- |
| Backend | Rust 2024, axum 0.8, sqlx 0.9 (MySQL), hexagonal layering (domain → application → infrastructure/api) |
| Frontend | React 19 + TypeScript + Vite, embedded with `rust-embed`; Tailwind v4, in-house components on Radix, React Router 7, TanStack Query 5 and Table, react-hook-form + zod, i18next (pt-BR default, en) |
| Database | MySQL 8, one database per company, never rows of several companies in one table; no `organization_id` anywhere |
| Tenancy | Single mode (`DATABASE_URL`) or multi mode (`CONTROL_DATABASE_URL`); see Multi-tenancy |
| IDs and money | UUID v7 in `CHAR(36)`; `DECIMAL(18,2)` + ISO currency, `rust_decimal` |
| Authentication | Web: opaque session in an `HttpOnly`/`SameSite=Lax` cookie, token hashed in the database. API: tokens `aphel_<prefix>_<secret>` with scopes in `Authorization: Bearer` |
| API | REST under `/api/v1`, every route documented with `utoipa`; the web generates its types with `openapi-typescript` |
| Events | Domain events written to an outbox in the business transaction, dispatched by the worker |
| Jobs | MySQL queue (`FOR UPDATE SKIP LOCKED`) + scheduler; `aphel serve`, `aphel worker`, `aphel serve --with-worker` |
| Files | `object_store` (local disk or S3-compatible), downloads only through authenticated endpoints |
| Email | `lettre` over SMTP; Mailpit in development |
| Architecture | Modular monolith, no microservices |
| Tests | No test suites, no per-commit tests. Validation is `cargo check`, `clippy`, `npm run typecheck`/`lint`, plus `make probe` (database-free platform invariants, under 30 s) after changes to access filters, hook order, the entity registry, operations or event actors. Behaviour that needs a database is checked by hand on a throwaway instance when the user asks for it |

## Working rules

- Code, comments, docs (`.md`), commit messages, API descriptions and JSON schemas in English. Every UI text through i18n in pt-BR and en.
- No explanatory comments or docstrings; only `TODO`/`FIXME` and directives tools require.
- Follow existing patterns and layering; read the neighbouring code before adding a new shape.
- Generated files are committed and regenerated, never edited by hand: `back/app/Cargo.toml`, `back/app/src/modules.rs`, `web/src/modules.generated.ts` and `@modules/<key>` aliases (`make modules`), `web/src/api/schema.d.ts`, module `api-types.d.ts` and `web/src/console/api-types.d.ts` (`make api-types`), `web/src/i18n/i18next.d.ts` (`npm run i18n:types`), `back/openapi.json` and `back/console-openapi.json` (`aphel openapi [--console]`), `docs/modules/hooks.md`, `docs/plugins/host-catalog.md`.
- **Schema policy:** no installation exists, so schemas are rewritten for fresh installs. No adoption of old migrations, legacy aliases, checksum exceptions or compatibility shims. Normal migration bookkeeping stays.
- **Architecture ratchet:** `cargo run -p xtask -- arch check` (part of `make lint`) compares counts with `back/xtask/arch-baseline.json` and fails when one rises; when a count falls, run `arch baseline` and commit it. `--details` lists the findings. Counts include kernel domain words, public kernel modules, plugin paths into the kernel, cross-plugin SQL and foreign keys, unscoped spawns and mutable statics.
- Public documentation goes to `web/docs/src/content/docs`; error codes are listed in `reference/errors.md`, settings in `admin/configuration.md`, commands in `admin/cli.md`.

## Platform rules

- Only the kernel and `wasm_runtime` (the plugin runtime, declared `.required()` in its manifest) are always on; neither appears among the modules a company or operator can toggle. Every other module is optional and blueprints declare their own required modules.
- `back/aphel.modules.toml` compiles the modules named by the build blueprints plus those marked `required = true` (only `wasm_runtime`); `make modules` drops any other module that no blueprint names. The arch check counts `required = true` entries, so do not add more.
- Modules and plugins depend on contracts, never on each other's internals. Each table has one owner; no SQL or foreign keys across owners. The kernel never reads a module's table and holds no domain words (the arch check counts them).
- Capability modules have generic names (`workflow`, `tasks`, `assignment`); domain constants live in contract crates.
- Kernel extension points ("core points") are contributed with `app.contribute::<dyn Trait>(..)`: `FileReferenceSource`, `HealthSource`, `ReferenceProvider`, `FieldTypeDefinition`, `RuntimeCatalogSource`, `SiteRequestHandler` and others, re-exported by `aphel_sdk::points`; capability traits are imported from their owner's contract.
- Open vocabulary: field types, reference kinds, entity traits (`EntityTraits`), service actors (`Actor::Service`, `AccessScope::acting_as`) and localized labels (`LocalizedLabels`, `locales/labels.json` per owner) are open catalogs, never closed enums.
- Native modules reach the kernel only through `aphel_sdk::platform::*` and the runtime through `aphel_sdk::Runtime` (`ctx.runtime()`, `env.runtime()`, `aphel_sdk::runtime()`). The arch check fails on `aphel_kernel::<a>::<b>` paths, `ctx.state()`/`runtime_state()`/`env.state()` and foreign dotted record keys in module code.
- `cargo check -p aphel --no-default-features` must build the binary with only the kernel.

## Kernel

### Modules

- Everything is a module (`trait Module` in `kernel/src/modules`: `manifest()`, `register(&mut AppBuilder)`, `on_enable`/`on_disable`/`uninstall`). The `AppBuilder` is the only contribution point: routes (`routes`, `public_routes`, `root_routes`, `app_routes`), hooks (`on`, `filter`, `declare_hook`), services (`provide`, `decorate`), permissions (`permission(..).grant_to([..])`), `entity`/`extend`, `job`, `consumer`, `schedule`, `worker`, `operation`, `public_event`, `notification_kind`, `widget`, `openapi_tag`, `contribute`.
- `build()` orders modules by dependencies (then key) and freezes the `App`; the binary installs it with `install_app` and everyone reads `aphel_kernel::modules::app()`. Contributions remember their module and are ignored when it is disabled (`EnabledModules`), including system fields and service layers.
- Module crates depend on `aphel-kernel` and on the modules they use by alias (`leads = { package = "aphel-mod-leads", .. }`); dependency cycles are forbidden, so two modules that need each other talk through a hook, a service, an operation or a typed contribution.
- Enablement lives in `installed_modules`; module migrations are `Migration::new(n, desc, include_str!(..))` in the manifest, applied in load order and recorded in `module_migrations`. A module declares `depends_on`/`optional` for every module whose tables it references.
- CLI: `aphel modules list|doctor|hooks [--markdown]|uninstall <key> [--purge]`; modules add subcommands through `ModuleCommand` (`aphel org`, `aphel search`).

### Access and authorization

- Every use case receives an `AccessScope` (`kernel/src/domain/access/access_scope.rs`: user, role, expanded teams, permissions, API token, automation, enabled extensions). System work uses `AccessScope::system()`; jobs that need an actor build it with `build_access_scope`.
- Authorization is an explicit call in the use case: `scope.require(&Permission::X)?` or `scope.record_filter(&Permission::X)?` (`Authorizer` in `kernel/src/application/access`). Handlers extract `AuthContext` (rejects invited/deactivated users and sessions pending 2FA) and pass its scope. There is no permission extractor.
- Permissions are constants in `kernel/src/domain/access/permission.rs` (core) or contract crates, registered with `app.permission(PermissionDefinition::record|administrative(..))`; `permission_catalog()` reads the `App`. `/api/v1/system/*` needs `system.manage`.
- Unsafe requests need `X-Requested-With: aphel` except with a bearer token and on `/api/v1/public/*` and `/api/v1/hooks/*`. Sessions in `aphel_session`; API tokens go through the same `AuthContext` with permissions = scopes ∩ creator's permissions.
- Field-level access: `FieldSet::for_scope` hides fields (`422 field_not_accessible` when filtered or sorted on), `FieldAccessPolicy` guards writes (`kernel/src/application/fields/permissions`), the `record_redactor` shapes responses. Personal data is flagged `is_personal`/`is_sensitive`; data subjects and retention live in `modules/privacy`. Account security events go through `state.security_events`.

### API conventions

- Item `{ data }`, list `{ data, meta: { total, page, per_page } }` via the pagination extractor; filters `filter[field][op]` (`RecordListQuery`), `include=` (`IncludeQuery`), optimistic locking with `If-Match` (`IfMatch` + `ensure_version`, rechecked in the transaction with `records::lock_version`).
- Every error is an `AppError` rendered as `{ "error": { "code", "message", "details" } }`; new codes go to `web/docs/src/content/docs/reference/errors.md`.
- Every route has a `utoipa` annotation and appears in `/api/openapi.json`.

### Mutations, events and jobs

- Every business mutation publishes a domain event and writes an audit entry in the same transaction: `begin` → write → `events().publish(&mut tx, DomainEvent::new(..))` → `audit().record(&mut tx, AuditEntry::new(..))` → `commit`. Reference: `change_member_role` in `kernel/src/application/organization_admin/members.rs`, and `modules/people/src/service.rs` for the `ctx.begin()` + `publish_mutation` style. Diffs use `application::audit::diff_changes` (sensitive values become `"[redacted]"`).
- Event consumers implement `EventConsumer` (stable `name()`, `accepts`, `consume`) and are registered with `app.consumer`; `outbox.dispatch` fans out one `event.consume` job per consumer (at least once, retried per consumer).
- Jobs implement `JobHandler` and are registered with `app.job`/`app.worker`; schedules with `app.schedule(ScheduledJobDefinition::new(kind, cron_with_seconds)?)`. Enqueue with `JobQueue::enqueue(&mut tx, Job::new(..))` inside the transaction. Handlers are idempotent.
- Events exposed to integrations enter the public catalog (`app.public_event`), rendered by `ApiPublicEventRenderer`. Long user processes finish by publishing an event with `requested_by` in the payload.

### Records and entities

- `EntityDefinition` unifies native, plugin and administrator entities; storage is native `table`, host-generated `managed` columns/indexes (unique indexes cover live rows only) or EAV `dynamic`. Keys are `<owner>.<entity>`; administrator objects are `custom.<key>` in `custom_records`. Generic routes: `/api/v1/entities/{entity}/records`.
- Every write goes through the `RecordPipeline` (`kernel/src/records`): authorization → `saving` (filter) → write + `CustomFieldWriter` → `validating` (veto) → `saved` → event + audit → commit → `committed`; deletions run `deleting`/`deleted` with `restrict`/`clear`/`cascade` reference policies. Each type has a `RecordTypeHandler` used by automations, imports and bulk actions; behaviour only some types have is a capability in `EntityDefinition`, never a `match` on the type.
- Listing, filtering, sorting and search only through `RecordQuery`/`RecordQueryEngine` with ASTs validated against the field registry (`kernel/src/application/fields/registry.rs`); custom values are read with `CustomFieldReader` and written with `CustomFieldWriter`. Code that receives a record type from a user never uses `RecordTypeRegistry::builtin()`: it asks `state.registry().await?` (`RecordTypeCatalog` in `kernel/src/application/custom_objects/catalog.rs`, invalidated through `custom_schema_version`).
- Types are added with `app.entity(def)` and extended with `app.extend(key, ..)` (system fields, relations, `managed_field`, `RecordTypeExtension::side_table`).

### Organization data and configuration

- Company data lives in `organization_settings` (single row `id = 1`, read through `OrganizationRepository::find()`); single-configuration tables use the same pattern. The company and its first administrator are created only by the installer (`/api/v1/installer`, or `install_organization` when provisioning a tenant); other users join by invitation.
- The environment holds infrastructure only (`HTTP_*`, `DATABASE_URL` or `CONTROL_DATABASE_URL` with `CONSOLE_HOST`, `TENANT_DATABASE_MAX_CONNECTIONS`, `TENANT_EXPORT_DIR`, `APP_ENCRYPTION_KEY`, `APP_ENCRYPTION_PREVIOUS_KEYS`, `RUN_MIGRATIONS`, `RUST_LOG`, `WORKER_*`, `SCHEDULER_INTERVAL_SECS`, `HEALTH_*`, `METRICS_TOKEN`, `HTTP_TRUSTED_PROXIES`, `LOGIN_LOCKOUT_*`, `WEBHOOKS_ALLOW_PRIVATE_NETWORKS`, `UPGRADE_MIN_FREE_DISK_MB`, `SMTP_*` fallback). Public address, upload limit, file storage, webhooks, token limits, catalog and SMTP are organization configuration edited in the UI (`OrganizationConfigCatalog` in `kernel/src/application/organization_profile/settings.rs`, `state.organization_config`), refreshed in every process and changed with an event and audit.
- Secrets only through `SecretVault` (`SecretKey { owner_type, owner_id, name }`, encrypted with `APP_ENCRYPTION_KEY`); never returned by the API, logged or audited in clear.
- Files only through `FileStorage` (`state.storage`, keys `<year>/<month>/<id>`) and authenticated endpoints; attachments reference records with `RecordRef`. `DynamicFileStorage` follows the organization's storage settings and, in multi mode, stores every key under `<tenant slug>/`.
- Email is sent with `state.mailer()`; without SMTP it fails with `mail_not_configured`. Email links use the saved public address (`PublicBaseUrl::app_link`, `Ctx::app_root_url`), never the request `Host`.

### Mounts

Aphel's own paths are `/api` and `/_aphel` (`/_aphel/assets`, `/_aphel/shared`, `/_aphel/docs`, `/_aphel/health`, `/_aphel/health/ready`, `/_aphel/metrics`). The app is mounted at `organization_settings.mounts.app.base_path` (`/` or one segment) and always at `/_aphel/app`; `aphel mounts reset` puts it back at `/`. `web::dispatch` (`kernel/src/api/web.rs`) resolves the mount, serves `app_routes` and falls back to `index.html` with `<meta name="aphel-base">`. On the web, links to app screens go through `appPath`/`appUrl` (`web/src/sdk/app-base.ts`).

## Multi-tenancy

Code in `back/kernel/src/tenancy`; guide `web/docs/src/content/docs/admin/multi-tenancy.md`.

- **Control plane:** `ControlPlane` owns the control database (`tenants`, `tenant_hosts`, `operators`, `operator_sessions`, `operator_audit`) and opens tenant pools on the same server; tenant databases are named `<control database>_<slug>`.
- **Routing:** `TenantDirectory` maps hosts to tenants (reloaded every 5 s and after console changes). `tenancy::router` sends `CONSOLE_HOST` to the console, active tenants to their `TenantRuntime` (pool, `AppState`, router, refreshers, tracked tasks) and answers `404 tenant_not_found` or `503 tenant_unavailable` otherwise.
- **Tenant scope:** tenant code runs inside a `task_local` scope (`tenancy::scope`, kept while response bodies stream). `runtime_state()`/`aphel_sdk::runtime()` resolve through it; in multi mode everything fails closed outside a scope.
- **Rules for new code:**
  - Tenant state lives in `AppState` or in `TenantLocal<T>` (`aphel_sdk::platform::tenancy::TenantLocal`: one value per tenant, one per process in single mode, `None` outside a scope), never in a plain `static`. Process-wide statics are only for code-level things (regexes, interned strings, HTTP clients, compiled plugin cache by hash, environment settings).
  - Spawned work uses `tenancy::spawn` (or `tenancy::bind` for a `JoinSet`), never a bare `tokio::spawn`.
  - The arch check counts `unscoped_spawns` and `mutable_statics`.
- **Worker:** `tenancy::run_workers` keeps one worker per active tenant (scheduler, lock recovery, heartbeat, outbox and jobs in the tenant database) over a process-wide `WORKER_CONCURRENCY` semaphore shared in turn.
- **Provisioning and CLI:** `tenancy/provisioning.rs` registers, provisions (database, migrations, `install_organization`), suspends, resumes, manages hosts and deletes (export with the `org` command, files by prefix, database) tenants, writing `operator_audit`. `aphel migrate` and boot migrations cover the control database and every tenant. CLI: `aphel tenants list|create|suspend|resume|delete|migrate`, `aphel console create-operator`, `aphel tenancy-check` (two throwaway tenants, fails on any leak). Commands acting on one company take `--tenant <slug>` and refuse to run without it in multi mode.
- **Console:** `tenancy/console` serves `/api/console/v1` (own OpenAPI) and the SPA on the `console` surface (`web/src/console`), with operator sessions in `aphel_console_session`, invitations by one-time link, tenant management, health, audit, the per-company plugin management switch (`tenants.plugin_management_enabled`) and operator management of a company's plugins and modules (`console/plugins.rs`, through the `PluginAdministration` core point contributed by `wasm_runtime`).

## Runtime plugins

Third parties ship `.aphelplugin` packages (`plugin.toml` + `backend.wasm` + `web/`), installed from the UI or API without rebuilding, handled by the required module `modules/wasm_runtime` (always enabled, not listed among modules). Guide: `web/docs/src/content/docs/plugins`.

- Contract: WIT world `aphel:plugin@1.0.0` in `modules/wasm_runtime/wit/plugin.wit` (copied to both plugin SDKs by `make modules`). `1.x` only adds functions; existing records, enums and signatures are frozen. No production exists yet, so changing the WIT and rebuilding plugins is allowed.
- The host API (`host.rs`) exposes records only through `RecordTypeHandler`/`RecordQueryEngine`, plus key-value, settings, secrets, outbound HTTP under the SSRF policy, clock, log, notifications, events, operations and files; never SQL. Calls run as actor ∩ plugin capabilities (`AccessScope::restricted`); events, schedules and public routes run as the installer (`AccessScope::through_plugin`).
- Manifests declare managed entities, fields, permissions, hooks, operations, events, routes, schedules, settings, automation actions, notifications, widgets, menus, site pages and UI plugins; installs show a consent dialog from a dry run and reconcile grants, fields and entities on upgrade.
- Writes from in-transaction hooks are refused (`plugin_write_in_hook`); retryable callbacks write only retry-safe changes or go through `defer-effect`.
- `wasmtime` component model with epoch timeouts and 64 MB per instance; compiled code cached by the SHA-256 of `backend.wasm` in process and in `plugins_compiled`. The registry (`registry.rs`) rechecks `plugins_state.version` every 2 s per tenant.
- Modules reach plugins through the plugin bridge (`kernel/src/modules/plugin_bridge.rs`): `app.operation`, `app.forward_hook`/`forward_filter_hook`. A new module operation or hook never changes the WIT. Catalog: `GET /api/v1/wasm-runtime/host-catalog`, `docs/plugins/host-catalog.md`.
- Web: enabled bundles load before the app router (`web/src/kernel/modules/wasm_runtime.ts`) and import shared modules through the import map (`/_aphel/shared/*.js`, built from `web/runtime` and `web/src/sdk/index.ts`). The public web SDK and `@aphel/ui` (`web/src/sdk/ui.ts`) only grow.

## Public site

The optional module `site` serves plugin pages at `organization_settings.mounts.site` (prefix `/s` by default, optional host). The kernel only knows the mount (`SiteRequestHandler`, `kernel/src/modules/site_mount.rs`); content comes from `SiteContentSource` contributions of `contracts/site`, built by `wasm_runtime` from `[site]` in `plugin.toml` and `site/*.html`. `site` renders MiniJinja templates inside its theme (`modules/site/theme`), reads records with exposure filters and field projections (`data.rs`), caches pages per tenant with tag invalidation (`cache.rs`), and provides SEO, sitemaps, redirects, themes, menus, languages, islands, customer accounts and previews. Guide: `web/docs/src/content/docs/plugins/site-pages.md`.

## Frontend

- The web is assembled by modules (`web/src/kernel/modules`): each `defineModule({ key, translations, register(app) })` in `web/src/modules/<key>` contributes routes, navigation, settings, record tabs, slots, filters, overrides, field types, widgets and commands, filtered by the modules enabled on the server. There are no central lists; a page never imports another module's tabs or actions.
- Platform screens live in `web/src/features/<feature>`, registered by the `kernel` web module. Record screens use the building blocks in `web/src/features/records`.
- API calls only through `web/src/api` (it unwraps `{ data }` envelopes; pages with `meta` stay whole) with generated types.
- Texts only through i18n: module namespace = key (`web/src/modules/<key>/i18n/<locale>.json`), kernel namespaces in `web/src/i18n/<locale>/<namespace>.json`.
- UI components in `web/src/components/ui` take no i18n, session or API; colors, radii, heights and shadows come from tokens (`web/src/sdk/tokens.css`). Forms use `Field`/`ToggleField`, never a bare label and input; structured values (CPF, CNPJ, CEP, phone, fiscal codes) use masked inputs. Dialogs, toasts and error boundaries come from `web/src/components/feedback`.
- Screens are designed for the person doing the job every day: grouped by task, keyboard-first, plain errors that say how to fix them, no technical identifiers in front of people.
- The console (`web/src/console`) starts instead of the app when the shell carries `<meta name="aphel-surface" content="console">`; it loads no plugins or modules.

## Commands

```bash
make up            # MySQL and Mailpit (docker compose)
make modules       # regenerate module registries from back/aphel.modules.toml
make dev-back      # backend on 127.0.0.1:3001 (cd back && cargo run -- serve --with-worker for the worker too)
make dev-web       # Vite with /api proxied to the backend
make lint          # modules and themes sync --check, arch check, cargo fmt --check, clippy -D warnings, oxlint, typecheck
make probe         # platform invariants without a database
make api-types     # web types from the tenant and console OpenAPI documents
make build         # release build (web + binary)
(cd web && npm run i18n:types)   # i18n types after adding or moving JSON files
(cd web && npx vite build)       # refresh the embedded app before testing a local binary
```

Local throwaway instances use their own database and a free port (3001 may be taken by other projects); `*.localhost` hosts resolve to 127.0.0.1 in Chrome, which makes multi mode testable with `CONSOLE_HOST=console.localhost`.

## Where to read more

- `web/docs/src/content/docs/start/architecture.md`: crates, layers, request path, pipeline, events, jobs.
- `web/docs/src/content/docs/admin/`: installation, configuration, CLI, operations, multi-tenancy, backup.
- `web/docs/src/content/docs/plugins/`: plugin manifest, host API, SDKs, site pages, UI plugins.
- `web/docs/src/content/docs/reference/`: REST API, errors, hooks, permissions, events, design tokens.
- `docs/modules/hooks.md` (generated hook catalog), `docs/plugins/host-catalog.md`, `docs/ui/tokens.md`.
