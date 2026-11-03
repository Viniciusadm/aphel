# Modular Aphel: restructuring plan (WordPress style, compiled into the binary)

> Third-party plugins no longer use this compiled mechanism: they are runtime `.aphelplugin` packages (WebAssembly backend + ES module web bundle) described in [`docs/plugins`](../plugins/README.md). This document describes the compiled modules of the core and official modules.

## Context

Aphel has 251 thousand lines of backend code in one crate (`back/src`) and 106 thousand in the frontend. It **already has an extension system**: the `aphel-extension-api` crate, 10 extensions in `back/extensions/*` behind `ext-<key>` features, and `web/src/extensions`. That system, however, is a side door:

- The business modules (people, deals, leads...) do not use this system. They are hand-stitched into the core.
- An extension can only **add** things: routes, widgets, actions, providers and new types. It cannot:
  - intercept a core operation before or during its execution;
  - change data in transit;
  - add fields to `contact`;
  - replace a behavior;
  - call another extension.

The goal is to invert this. The core becomes a **minimal kernel**: platform and extensibility mechanisms. **All** business logic becomes modules, built with the same APIs that a third-party plugin uses. Plugins can interfere with each other through hooks, filters and overrides.

Decisions made with you:

- **Plugins compiled into the binary.** A plugin is a Rust crate plus a web package, listed in a build manifest.
- **Direct access.** A plugin imports the kernel and other modules as regular crates. The `aphel-extension-api`/`HostServices` layer ceases to exist.

---

## 1. Diagnosis: what currently prevents adding, changing or removing modules

| # | Coupling point | Where | Effect |
|---|---|---|---|
| 1 | 94 `.merge(x::router())` calls and 82 OpenAPI tags by hand | `api/routes/v1/mod.rs`, `api/docs.rs` | Every new route requires editing the core |
| 2 | 71 `with_handler`/`with_consumer`/`with_schedule` calls | `worker.rs`, `cli/worker.rs` | Jobs and consumers are tied to the core; `with_email_sending` calls `api::routes::v1::sequences` (layer inversion) |
| 3 | Closed or fixed `builtin()` lists | `fields/builtin.rs`, `automations/triggers.rs`, `actions.rs`, `dashboard/widgets.rs`, `config_packages/ports.rs`, `integrations/registry.rs`, `reports_builtin.rs` | They are open in shape, but fixed in code |
| 4 | **Closed** catalogs | `permission.rs` (`permission_catalog()`), `public_events/catalog.rs` (`PublicResource` enum, 50 static events), `domain/notifications/kind.rs` (enum with 19 variants duplicated in the DTO) | Nothing can be added without editing the core |
| 5 | `match record_type { "contact" => ... }` in about 20 files | `api/automations.rs` (3 blocks), `imports`, `exports`, `bulk_actions`, `privacy`, `org_export`, `notifications` | A new record type requires editing 20 files |
| 6 | `AutomationGateway` in the `api` layer | `api/automations.rs`, which imports DTOs and `*_use_cases` from 10 routes | It is the strongest coupling between modules |
| 7 | The only "before" mechanism is the fixed `record_write_guard()` chain | `infrastructure/fields/ports.rs:31` | It has only four fixed steps: Approval → StageChecklist → ValidationRule → LifecycleStage. It can only veto: it does not change values and does not accept contributions |
| 8 | "After" hooks are asynchronous only | `EventConsumer` via outbox | They run after the commit, with no power to veto or mutate |
| 9 | 84 `*Ports` traits with associated types bound to `Tx`, plus 83 `MySql*Ports` impls | `application/*/ports.rs`, `infrastructure/repositories/*` | Replacing a behavior requires rewriting the generic bundle. Each module carries 35 to 50 lines of getters |
| 10 | Migrations in a single folder, with `NNXX_` ranges per session | `back/migrations` (195 files) | The module does not own its schema and cannot be removed |
| 11 | Default permission grants in SQL migrations | 9 `*_grant_*_permissions.sql` files | Module permissions depend on SQL in the core |
| 12 | Extensions restricted to `ext_<key>_*` tables | `extensions/migrations.rs` (`foreign_tables()`) | It is not possible to `ALTER TABLE people` or extend an entity |
| 13 | Web: central lists | `app/routes.tsx` (25 imports), `layout/navigation.ts` (`NAVIGATION_ITEMS`), `settings/lib/sections.ts` and `settings/routes.tsx`, `i18n/index.ts` (85 JSON files and `NAMESPACES`) | Every new screen or translation requires editing the core |
| 14 | Web: screens compose tabs through direct imports | `ContactDetailPage.tsx` uses `useLifecycleTab`, `useScoringTab`, `withRelationsTab`... | A module cannot inject anything into another without editing the file |
| 15 | Web: closed field types | `switch` in `components/fields/FieldInput.tsx:43` and `records/display/FieldValueDisplay.tsx:85` | There is no plugin field type |
| 16 | Web: extensions register translations and renderers on load | `extensions/registry.ts` | This happens even when the extension is disabled |

Already exists and will be **reused**:

- `RecordTypeRegistry` (open) and `CustomFieldWriter`.
- The `RecordQuery` engine.
- The outbox and the `EventConsumer`.
- `installed_extensions`, with enabling, semver dependencies, settings, data export and purge.
- `extension_migrations`, with checksums.
- The adapters in `back/src/extensions/adapters.rs`.
- On the web, the `registerRecordResource`, `registerTimelineRenderer` and `registerCommands` registries, the `WebExtension` contract and the `ExtensionGate`.

---

## 2. Proposed architecture

### 2.1 Pieces

```
back/
  kernel/            crate aphel-kernel  (platform + extension mechanisms)
  modules/<key>/     crates aphel-mod-<key> (all business logic, including what is core today)
  plugins/<key>/     third-party crates (same format; separate folder for organization only)
  app/               crate aphel (binary): main, CLI, generated module registry
  aphel.modules.toml list of what goes into the build
web/src/
  kernel/            shell, router, module registry, slots, filters, base ui
  modules/<key>/     one package per module (index.ts + screens + i18n)
```

**A single concept: module.** The modules that ship with Aphel, the former "extensions" and third-party plugins have the same format, the same API and the same lifecycle. There is no "business core": `people` is a module like any other, it just comes enabled by default.

**The kernel (`aphel-kernel`) contains only:**

- Infrastructure: config, database (`MySqlTx`), `AppError`, HTTP server (`AuthContext`/`SessionContext` extractors, CSRF, rate limiting, error envelope, OpenAPI and embedded web).
- Identity: users, sessions, 2FA, tokens, roles, teams, **open permission registry**, `AccessScope`, security events.
- Installation and `organization_settings`, plus file storage, the `SecretVault` and the SMTP transport.
- Events, outbox and audit; job queue, scheduler and `WorkerRuntime`.
- The **record engine**: `RecordTypeRegistry`, custom fields, `RecordQuery`, layouts, views, custom objects and relationships, search index.
- The **module runtime**: `Module`, `AppBuilder`, `HookBus`, `Services`, per-module migrations, `installed_modules` and the open catalog of events and notifications.

**Modules** are everything else. Some are business modules:

- `people`, `organizations`, `deals`, `workflow`, `tasks`, `notes`;
- `leads` (the current `lifecycle`), `scoring`, `lead_distribution`;
- `products`, `price_lists`, `quotes`, `approvals`, `deal_items`, `renewals`, `currencies`;
- `email`, `sequences`, `scheduling`, `calls`.

Others are platform modules, generic over any record type:

- `timeline`, `tags`, `followers`, `record_sharing`, `favorites`, `reminders`, `trash`, `duplicates`;
- `imports`, `exports`, `bulk_actions`, `privacy`, `org_export`;
- `automations`, `webhooks`, `integrations`, `reports`, `dashboard`, `goals`, `forecast`, `config_packages`, `demo`.

The 10 current extensions (`ai`, `esign`, `lead_forms`, etc.) become modules.

### 2.2 Module contract: a `register` function, `add_action` style

The current `Extension` trait has about 20 methods. It is replaced by a manifest and a registration function, which is the equivalent of a WordPress plugin's main file:

```rust
pub trait Module: Send + Sync + 'static {
    fn manifest(&self) -> Manifest;
    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError>;
    async fn on_enable(&self, _ctx: &mut ModuleCtx<'_>) -> Result<(), AppError> { Ok(()) }
    async fn on_disable(&self, _ctx: &mut ModuleCtx<'_>) -> Result<(), AppError> { Ok(()) }
    async fn uninstall(&self, _ctx: &mut ModuleCtx<'_>) -> Result<(), AppError> { Ok(()) }
}
```

`Manifest` reuses the current `ExtensionManifest`:

- key, version, `requires_aphel`;
- `depends_on(key, semver)` and `optional(key)`;
- `settings`;
- `migrations: include_dir!("migrations")`.

The `AppBuilder` is the only contribution point. Each call is tagged with the module key, for gating and diagnostics.

| Contribution | API |
|---|---|
| Routes | `app.routes(OpenApiRouter)` mounts under `/api/v1/...` (no forced prefix); `app.public_routes(..)` |
| Record type | `app.entity(def).handler(ContactHandler)` |
| Extend an existing type | `app.extend("contact", \|t\| t.system_field(..).relation(..).computed(..))` |
| Code-defined field (no migration) | `app.extend("contact", \|t\| t.managed_field(FieldSpec::number("lead_temperature")))`: definition synced at boot, value stored in the EAV, locked in the UI |
| Synchronous hook (action or veto) | `app.on(H, priority, handler)` |
| Filter (transforms a value) | `app.filter(F, priority, handler)` |
| Replace a service | `app.provide::<dyn LeadConverter>(Arc::new(MyConverter))`: the last provider wins |
| Permissions | `app.permission(Permission::record("leadpro.read")).grant_to(["admin", "manager"])`: granted on enable, with no grant migration |
| Public events, notifications | `app.public_event(..)`, `app.contribute(Arc::new(NotificationKindDefinition::new(..)))`: open catalogs that replace the enums |
| Worker | `app.job(handler)`, `app.consumer(consumer)`, `app.schedule(kind, cron)` |
| Automations, widgets, integrations, config sections, providers (mail, telephony, login, calendar) | `app.trigger(..)`/`app.action(..)` through `AutomationBuilder`; widgets, integrations, configuration sections and providers use `app.contribute::<T>(Arc::new(..))` with owner-defined types |
| OpenAPI tag | `app.openapi_tag(name, description)` |

`AppBuilder::build()` does the following:

1. Sorts the modules by the dependency graph.
2. Calls `register` on each one.
3. Freezes the result into an `App`, which lives in the `AppState` (`state.app`) and in the worker.

`AppState` keeps 18 platform fields plus `app`. `serve` and the worker build the same `App`, which eliminates the worker's fake `AppState`.

**Enabling or disabling without restarting** works like it does today with extensions. Everything that was compiled registers itself. Each contribution stores the module key, and dispatch, whether of a hook, route, consumer, job or menu, checks the cached enabled set (`installed_modules`, renamed from `installed_extensions`, invalidated on write). Modules marked `required` in the build manifest, such as `people`, cannot be disabled. The current dependency check still applies.

### 2.3 Hooks: actions, filters and vetoes, typed and synchronous

The `HookBus` lives in the kernel and is **keyed by Rust type**, with a list ordered by priority (lower runs first, default 10, as in WordPress). A hook is a type, and the compiler guarantees the payload:

```rust
pub trait Hook: 'static { type Args<'a>; }

app.on(RecordSaving, 10, |ctx, args| Box::pin(async move { ... }));
app.filter(RecordQueryFilter, 20, |ctx, plan| Box::pin(async move { Ok(plan) }));
```

The handler receives a `HookCtx` with the following fields:

- `tx: &mut MySqlTx`, the same transaction as the operation;
- `scope: &AccessScope`;
- `services: &Services`, with access to any service of the kernel or of another module;
- `module`, the key of whoever triggered it.

Returning `Err` aborts the whole operation, which provides the veto. Errors go out through `AppError`.

**Generic record hooks**, valid for any type (core, modules and custom objects), live in the kernel's write pipeline:

| Hook | When | Can |
|---|---|---|
| `RecordSaving` (filter) | Before the `CustomFieldWriter`, inside the transaction | Change `values` and fields, fill in defaults, normalize |
| `RecordValidating` (veto) | After writing and before the event | Reject with 422 or 409. Replaces `RecordWriteGuard`: approval, checklist, validation rule and lifecycle become hooks registered by the modules themselves |
| `RecordSaved` (action) | After writing, in the same transaction | Write its own data atomically |
| `RecordCommitted` (action) | After the commit, in-process | Quick side effects and cache |
| `RecordDeleting` / `RecordDeleted` | Deletion | Block, cascade. Replaces `ensure_deletable` |
| `RecordQueryFilter` (filter) | In the `RecordQueryEngine` | Add a condition or ordering to the `QueryPlan`, for example to hide records |
| `RecordSerializing` (filter) | Before the `record_redactor` | Add or hide keys in the response |
| `RecordListColumns` / `RecordSchema` (filter) | `/fields/{type}/schema` endpoints | Change the schema the web receives |

**Specific hooks** live in each module. They are public types in the module's crate, and other modules use them through a Cargo dependency. Examples:

- `leads::LeadQualifying` and `leads::LeadConverting { deal_values }` (filter over the values of the created deal);
- `deals::DealWinning`;
- `email::OutgoingEmail` (filter over the message);
- `automations::ActionRunning`.

**Rule for module authors:** every use case that mutates state goes through the record pipeline, which fires the generic hooks. Relevant decision points of the module also expose a typed hook or filter of their own.

**Domain events remain asynchronous.** They live in the outbox, with at-least-once delivery, and serve integrations, automations, search and webhooks. The split is this:

- **Hook**: synchronous and transactional, can change or veto the operation.
- **Event**: asynchronous and durable, to react to what has already happened.

The event catalog is open (`app.public_event`), and `PublicResource` is no longer an enum.

### 2.4 Behavior overrides

There are three levels, from the finest to the coarsest:

1. **Filter:** changes data in the middle of an existing flow.
2. **Replaceable service:** each module exposes the swappable parts as `Arc<dyn Trait>` in the `Services` container, for example:
   - `people::DuplicateDetector`;
   - `leads::LeadConverter`;
   - `deals::AmountCalculator`;
   - `email::Sender`.

   Callers always resolve through the container (`services.get::<dyn LeadConverter>()`), and `app.provide` from another module replaces the implementation. The original remains accessible for decoration: `app.decorate::<dyn LeadConverter>(|inner| Arc::new(Wrap(inner)))`.
3. **Route:** not overridable; a plugin offers its own route instead.

When two contributions compete for the same thing, **the last one in load order wins**: dependencies first, then priority and the order in `aphel.modules.toml`. The conflict is logged and shown in `aphel modules doctor`. Nothing is forbidden.

### 2.5 The simplification that unlocks everything: goodbye to generic `*Ports`

The 84 `*Ports` traits with associated types bound to `Tx` were made to inject fakes in tests, which no longer exist. They prevent hooks and swappable services, because each bundle is closed over its type. Replacement:

- A use case becomes a function or struct that receives `&Ctx`, that is, `Services` + `AccessScope` + `&mut MySqlTx`, and calls concrete repositories, `ctx.events()`, `ctx.audit()`, `ctx.hooks()` and `ctx.fields()`.
- The `*Repository` traits with `type Tx`, the `*Ports` and the `MySql*Ports` with getters go away. Repositories become structs with functions over `&mut MySqlTx`.
- The hexagonal architecture stays **inside each module**, in a lightweight form: pure `domain.rs`, `service.rs` with the use cases, `store.rs` with the SQL and `http.rs` with the routes. It is no longer a global `domain/ application/ infrastructure/ api/` tree.
- The "mutation = event + audit in the same transaction" contract remains, now guaranteed by the kernel's record pipeline (`records::save(ctx, write)`), which publishes, audits and runs the hooks.

### 2.6 Record types as handlers: the end of `match "contact"`

`EntityDefinition` gains a `RecordTypeHandler`, an object with generic operations over JSON:

```rust
#[async_trait]
pub trait RecordTypeHandler: Send + Sync {
    async fn create(&self, ctx: &mut Ctx<'_>, values: Value) -> Result<Value, AppError>;
    async fn update(&self, ctx: &mut Ctx<'_>, id: Uuid, values: Value) -> Result<Value, AppError>;
    async fn delete(&self, ctx: &mut Ctx<'_>, id: Uuid) -> Result<(), AppError>;
    async fn load(&self, ctx: &mut Ctx<'_>, id: Uuid) -> Result<Option<Value>, AppError>;
}
```

This handler is then used by:

- automations (`AutomationGateway`, which leaves `api/` and moves to the `automations` module);
- import, export, bulk actions, privacy, `org_export`, notifications, `lead_forms` and incoming webhooks.

The roughly 20 `match` blocks disappear. A new plugin type automatically gets import, export, bulk actions, automation, search, trash and webhooks. Custom objects use a generic handler.

### 2.7 Extending an existing entity

There are four ways, none of which touch the owning module:

1. **Code-managed field (EAV):** `managed_field(..)`. No migration needed. It shows up in forms, filters, columns, imports and the API automatically, because everything is schema-driven.
2. **Own column:** the plugin's migration can do `ALTER TABLE people ADD COLUMN ...` (the `ext_<key>_` restriction is dropped), and `extend` declares a `SystemField` over that column. It is faster for filtering and sorting.
3. **1:1 or 1:N side table** (`leadpro_contact_insights` with an FK to `people`), plus `relation(..)` or `computed(..)` on the type. This makes the fields filterable via `EXISTS` in `RecordQuery`.
4. **Behavior:** hooks and filters over the type, plus Rust *extension traits* over the module's public types (`impl LeadProExt for people::Contact`) for "new methods".

### 2.8 Dependencies between modules

- **At compile time:** a regular Cargo dependency (`aphel-mod-leads = { path = ... }`). The plugin uses the other module's public types, services, hooks and functions directly.
- **At run time:** `depends_on("leads", "^1")` in the manifest, for load order, blocking disablement and semver checks, as today. An optional dependency uses a Cargo feature or `app.has_module("deals")`.
- Each module decides what is `pub`. "Do not depend on internal details" is the author's convention, not a prohibition: if needed, the plugin can use any public item.

### 2.9 Per-module migrations

- Each module has a `migrations/` folder with `0001_*.sql`... numbered **within the module**. The global `NNXX_` ranges cease to exist.
- The runner is the current `extensions/migrations.rs`, generalized. The `module_migrations (module_key, version, checksum)` table applies everything in topological dependency order. The kernel is the `kernel` module.
- The prefix restriction goes away. Naming tables `<key>_*` is recommended, but not required.
- `uninstall` + optional `migrations/uninstall.sql` (`aphel modules uninstall <key> --purge`) reuse the data export and purge that already exist.
- Since there is no production, the current 195 migrations are **rewritten and consolidated per module**, one to three per module, with no compatibility.
- Default permission grants leave SQL and move to `grant_to(..)`, applied on enable.

### 2.10 Build selection

`back/aphel.modules.toml` lists the modules and plugins:

```toml
[modules]
people = { path = "modules/people", required = true }
leads = { path = "modules/leads" }
lead_pro = { path = "plugins/lead_pro" }
lead_pro_git = { git = "https://github.com/acme/aphel-lead-pro", tag = "v1.2.0" }
```

`cargo xtask modules sync` (or `make modules`) generates two things:

- **On the backend:** the dependencies in `app/Cargo.toml` and `app/src/modules.rs` (`vec![Box::new(people::Module), ...]`).
- **On the web:** `web/src/modules.generated.ts` with the web packages of the same modules and the path of each one.

Installing a plugin means adding a line and running `make build`. Removing it means deleting the line. No kernel file is edited.

---

## 3. Frontend

New `web/src/kernel/modules`. The current `WebExtension` becomes `defineModule`:

```ts
export default defineModule({
  key: 'lead_pro',
  translations: { 'pt-BR': ptBR, en },
  register(app) {
    app.routes([{ path: 'lead-pro', lazy: () => import('./pages/Board') }])
    app.nav({ to: '/lead-pro', label: 'nav.board', icon: Flame, permission: 'lead_pro.read' })
    app.settings('organization', { path: 'lead-pro', label: 'settings.title', lazy: () => import('./settings') })
    app.recordTab('contact', { key: 'temperature', label: 'tab', render: TemperatureTab })
    app.slot('record.header.actions', { recordTypes: ['contact'], render: RecalculateButton })
    app.fieldType('temperature', { Input: TemperatureInput, Display: TemperatureBadge })
    app.filter('records.formLayout', (layout, { recordType }) => recordType === 'contact' ? withTemperature(layout) : layout)
    app.override('leads.ConvertDialog', LeadProConvertDialog)
    app.timelineRenderer('lead_pro.temperature_changed', TemperatureItem)
  },
})
```

What changes on the web:

- **Routes, navigation, settings and translations** come from the registries. The lists in `app/routes.tsx`, `NAVIGATION_ITEMS`, `settings/lib/sections.ts`, `settings/routes.tsx` and `NAMESPACES` go away. Each module uses its own namespace, and a script generates the key typings (`i18next.d.ts`) from the JSON files of all modules.
- **Named slots** (`<Slot name="..." props={..} />`) in the generic blocks of `features/records`:
  - detail header and actions, field panel, tabs, list row and bulk actions, form sections, sidebar;
  - dashboard, note or email composer, user menu.
- **Overridable components** (`<Overridable id="people.DetailPage" .../>`) in the main screens of each module.
- **Frontend filters** (`applyFilters('records.columns', cols, ctx)`), for columns, form layout, menu items and command palette commands.
- **Field type registry:** `FieldInput` and `FieldValueDisplay` query the registry instead of the `switch`. Native types are registered by the kernel.
- Module pages (`ContactDetailPage`) no longer import tabs from other modules. `leads` and `scoring` register their tabs on `contact`.
- **Gating:** contributions are registered for all compiled modules, but **filtered** by the enabled set (`/api/v1/modules`). This fixes the registration of translations and renderers of disabled extensions.
- **API:** still a `schema.d.ts` generated from the OpenAPI of the assembled binary, which already includes everything that was compiled. The client optionally gains typed `paths`.

---

## 4. Concrete example: the `lead_pro` plugin extending Lead

In Aphel, **a Lead is a contact** with `lifecycle_stage = lead`. Today this concept belongs to `application/lifecycle` and becomes the `leads` module. It exposes:

- the public type `leads::Lead`, which is a view over the contact;
- the hooks `LeadQualifying`, `LeadConverting` and `LeadDisqualifying`;
- the `dyn LeadConverter` service.

The plugin extends all of this without editing `people` or `leads`.

```
back/plugins/lead_pro/
  Cargo.toml          aphel-kernel, aphel-mod-people, aphel-mod-leads, aphel-mod-deals
  migrations/0001_lead_pro.sql
  src/lib.rs  hooks.rs  scoring.rs  converter.rs  http.rs
web/src/modules/lead_pro/   index.ts  pages/  components/  i18n/
```

`migrations/0001_lead_pro.sql`:

```sql
ALTER TABLE people ADD COLUMN lead_temperature TINYINT NULL, ADD INDEX idx_contacts_lead_temperature (lead_temperature);
CREATE TABLE lead_pro_touches (id CHAR(36) PRIMARY KEY, contact_id CHAR(36) NOT NULL, kind VARCHAR(32) NOT NULL, created_at DATETIME(3) NOT NULL, FOREIGN KEY (contact_id) REFERENCES people(id) ON DELETE CASCADE);
```

`src/lib.rs`:

```rust
pub struct LeadPro;

impl Module for LeadPro {
    fn manifest(&self) -> Manifest {
        Manifest::new("lead_pro", "1.0.0")
            .requires_aphel("^1")
            .depends_on("leads", "^1")
            .depends_on("deals", "^1")
            .migrations(include_dir!("$CARGO_MANIFEST_DIR/migrations"))
            .setting(Setting::number("hot_threshold").default(70))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.permission(Permission::record("lead_pro.read")).grant_to(["admin", "manager", "sales"]);

        app.extend("contact", |t| {
            t.system_field(SystemField::new("lead_temperature", "Temperatura", FieldType::Number, "lead_temperature").read_only())
                .relation(RecordRelation::has_many("touches", "Interações", "lead_pro_touches", "contact_id"))
        });

        app.filter(RecordSaving, 20, hooks::default_temperature);
        app.on(RecordValidating, 10, hooks::block_unqualified_owner_change);
        app.on(leads::LeadQualifying, 10, hooks::require_temperature);
        app.filter(leads::LeadConverting, 10, hooks::enrich_deal_values);
        app.decorate::<dyn leads::LeadConverter>(|inner| Arc::new(converter::WithTouches(inner)));

        app.consumer(scoring::TouchConsumer);
        app.schedule("lead_pro.cooldown", "0 0 3 * * *").job(scoring::Cooldown);
        app.public_event(PublicEvent::new("lead_pro.temperature_changed", "contact"));
        app.contribute(Arc::new(notifications::domain::NotificationKindDefinition::new(notifications::domain::NotificationKind::from_static("lead_pro.lead_hot"))));
        app.routes(http::router());
        app.openapi_tag("lead_pro", "Gestão avançada de leads");
        Ok(())
    }
}
```

`src/hooks.rs`, an excerpt:

```rust
pub async fn require_temperature(ctx: &mut HookCtx<'_>, args: &leads::LeadQualifyingArgs<'_>) -> Result<(), AppError> {
    let threshold = ctx.settings::<i64>("lead_pro", "hot_threshold")?;
    match people::store::get_column::<i64>(ctx.tx(), args.lead.id, "lead_temperature").await? {
        Some(value) if value >= threshold => Ok(()),
        _ => Err(AppError::unprocessable("lead_not_hot_enough")),
    }
}
```

What the developer gets without touching anything else:

- `lead_temperature` shows up in the form, the list, the filter, import and export, the API and automations, because everything is driven by the schema and the handler.
- Lead qualification now requires a temperature.
- Conversion creates the deal with the enriched values.
- The original converter is decorated.
- On the web, there is a tab, a header button, a field type, a menu item and a setting.

To remove it, just take the line out of `aphel.modules.toml`. To clean up the database, use `aphel modules uninstall lead_pro --purge`.

---

## 5. Migration strategy (sessions 29–36, already executed)

The order is designed to keep the binary compiling after every session. First the mechanisms live **inside the current crate** (`back/src/modules/<key>`). The crates are only split at the end, once the cycles have been cut.

| Session | Type | Content |
|---|---|---|
| **29 Module kernel** | Foundation | `Module` trait, `Manifest`, `AppBuilder`, `App`, `HookBus` (action, filter and veto with priority), `Services` (provide and decorate). Open catalogs of permissions (with `grant_to`), public events (the `PublicResource` enum goes away), notification kinds (the enum goes away), OpenAPI tags, routes and worker parts. `installed_extensions` → `installed_modules`; `extension_migrations` → `module_migrations`. Create `aphel modules list\|doctor\|uninstall` |
| **30 Record pipeline** | Foundation | `records::save/delete` in the kernel, with the hooks `RecordSaving`, `RecordValidating`, `RecordSaved`, `RecordCommitted`, `RecordDeleting`, `RecordQueryFilter` and `RecordSerializing`. The `record_write_guard()` chain becomes hooks registered by approvals, workflow, validation_rules and leads. `RecordTypeHandler` and `extend`/`managed_field`. The ~20 `match "contact"` blocks switch to the handler. `AutomationGateway` leaves `api/` |
| **31 Pilot: people, organizations, leads, scoring** | Foundation | Move to `src/modules/<key>/` in the new pattern (concrete `Ctx`, no `*Ports`), with migrations consolidated per module, `Lead*` hooks and the `LeadConverter` service. Validates the design before scaling |
| **32 Web kernel** | Foundation + parallel | `defineModule`, registries for routes, menu, settings and i18n, `Slot`, `Overridable`, `applyFilters`, field type registry, gating via `/api/v1/modules`. Convert the 10 web extensions and the pilots |
| **33 Revenue modules** | Parallel (4) | deals, workflow, deal_items, renewals, currencies · products, price_lists, quotes, approvals · tasks, notes, reminders, calls · forecast, goals |
| **34 Communication and platform modules** | Parallel (4) | email, sequences, scheduling · automations, webhooks, integrations · imports, exports, bulk_actions, org_export, privacy · timeline, tags, followers, record_sharing, favorites, trash, duplicates, search, dashboard, reports, config_packages, demo |
| **35 Former extensions become modules** | Parallel (3) | The 10 crates in `back/extensions` switch to `Module`/`AppBuilder` with direct access. Remove `aphel-extension-api`, `HostServices`, `adapters.rs` and the in-process `host.api` |
| **36 Crate split and build** | Foundation | `back/kernel`, `back/modules/*`, `back/app`; `aphel.modules.toml` + `make modules` (generates the back and web registries); example plugin `back/plugins/lead_pro` + `web/src/modules/lead_pro`; `docs/modules/` (guide, hook catalog generated by `aphel modules hooks`, slots and filters); update `CLAUDE.md`, `back/README.md`, `docs/extensions` → `docs/modules` |

**What is removed:**

- `aphel-extension-api` and `HostServices`.
- The generic `*Ports`, `*Repository` traits and `MySql*Ports`.
- The fixed `builtin()` lists, the closed `permission_catalog()`, the `NotificationKind` and `PublicResource` enums, the `NNXX_` ranges, the grant migrations and the `ext_` prefix restriction.
- `api/routes/v1/mod.rs` with 94 merges and `worker.rs` with 71 registrations.
- On the web: central lists of routes, menu, settings and namespaces, and the cross imports of tabs.

**What is rebuilt:**

- The record write pipeline.
- The `AutomationGateway`.
- The migrations, consolidated per module.
- The worker, which is now built from the `App`.
- The web navigation and settings.

**What is reused almost intact:**

- The fields and query engine.
- The outbox, jobs and scheduler.
- The lifecycle of enabling, dependencies, settings, and export and purge.
- The web's `records/*` and the timeline and command registries.

**Validation in every session** (no tests, per the project rule):

- `cargo fmt`, `cargo clippy -D warnings` (`make lint`) and `npm run typecheck`/`lint`;
- `make api-types` to regenerate the types;
- `aphel migrate` on a clean database (`make down && make up`);
- `aphel serve --with-worker` + `aphel demo seed`, and manual checking of the affected screens.

**End-to-end validation in session 36 with `lead_pro`:**

1. It goes into `aphel.modules.toml`, through `make build`, and the migration applies.
2. The field shows up in the contact form and filter.
3. Qualifying a cold lead returns 422 `lead_not_hot_enough`.
4. Conversion carries the enriched values.
5. When disabled, the routes respond 404, the menu disappears and the hooks stop.
6. After removing the line and rebuilding, the binary starts without the plugin.

---

## 6. Trade-offs and limitations

- **Installing requires recompiling.** There is no "plugin upload" like in WordPress. The Docker image has to be rebuilt with the desired `aphel.modules.toml`. That is the price of total freedom with native code. WASM was left out.
- **Plugin = trusted code.** A plugin has full access to the database, the secrets and the process. There is no sandbox: security means choosing what goes into the build.
- **Conflicts are possible by design:**
  - two filters on the same field;
  - two overrides of the same service or route, where the last one wins;
  - a concurrent `ALTER TABLE`.

  The mitigation is diagnostic, not restrictive: the conflict log, `aphel modules doctor`, the list of who registered each hook and the explicit priority.
- **Compatibility:** without a separate stable contract, any public change to a module can break plugins. The upside is that the breakage shows up **at compile time**, not in production. Module semver and `requires_aphel` communicate this. It is worth keeping hook types and public services stable and changing internals freely.
- **Synchronous hooks inside the transaction:**
  - a slow or buggy plugin stalls or rolls back the core operation;
  - heavy hooks should go to events or jobs (documented);
  - there is no per-hook timeout.
- **`ALTER TABLE` on another module's table** couples the schema. Removing the plugin requires an `uninstall.sql` to clean up columns. Without it, the column is left orphaned, but it is harmless.
- **The end of generic `*Ports`** no longer allows fakes for unit tests. Today this does not matter, because tests are forbidden. When they come back, the natural path is integration tests with MySQL.
- **Size of the migration:** about 250 thousand lines reorganized in eight sessions. The biggest risk is in sessions 30, 31 and 36, which cut cycles between modules (for example email ↔ sequences ↔ quotes in the worker). The pilot exists to fix the design before scaling.
- **Enabling and disabling at run time** keeps the cost of checking the enabled set on every dispatch, with an in-memory cache invalidated on write. Disabling `people` is blocked by `required` and by dependencies.
- **Web:** everything stays in the main bundle, with `lazy` pages. A disabled module still adds to the download, but executes nothing.

---

## Steps for approval

1. **Session 29:** module kernel (`Module`/`AppBuilder`/`HookBus`/`Services`, open catalogs, `installed_modules`, `module_migrations`).
2. **Session 30:** record pipeline with hooks and `RecordTypeHandler`. The end of per-type `match` blocks and of `AutomationGateway` in `api/`.
3. **Session 31:** pilot of people, organizations, leads and scoring in the new pattern, without `*Ports`.
4. **Session 32:** web kernel (`defineModule`, slots, overrides, filters, field types, registries for routes, menu, settings and i18n).
5. **Sessions 33–34:** migration of the remaining modules in parallel.
6. **Session 35:** former extensions as modules; removal of `aphel-extension-api`.
7. **Session 36:** crate split, `aphel.modules.toml`, example plugin `lead_pro` and documentation.

Execution: sessions 29 to 36.
