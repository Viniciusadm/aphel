# Runtime plugins

A runtime plugin is a `.aphelplugin` package that an administrator uploads in **Settings › Integrations › Modules › Plugins** (or `POST /api/v1/plugins`). Installing, upgrading, enabling, disabling and uninstalling a plugin needs **no rebuild, no restart and no change to the Aphel repository**: the plugin lives in its own repository, and the running server (and every worker) picks it up within a few seconds.

| | Compiled modules ([`docs/modules`](../modules/README.md)) | Runtime plugins (this guide) |
| --- | --- | --- |
| Who | Aphel core and official modules | Third parties |
| Install | Line in `back/aphel.modules.toml` + rebuild | Upload of a `.aphelplugin` file, after a consent screen |
| Backend | Rust with full access (`Ctx`, SQL, every hook) | A WebAssembly component (Rust via the SDK, or any language with component tooling), sandboxed, restricted to the permissions it declares |
| Data | Own tables and migrations | Entities (managed custom objects), managed fields, key-value storage and secrets |
| Web | Built with the Aphel web app | Prebuilt ES module loaded at runtime through an import map |

Reference plugins, kept in their own repositories next to Aphel:
- `aphel-equine`, a stud farm. It adds horses with pedigree and sanitary exams, plus a field on deals. A rule blocks winning a deal with expired exams, and a reaction transfers the horse when the deal is won.
- `aphel-probe`, which exercises every host call and hook kind. It is used as an end-to-end check of the platform.

## Package

```
plugin.toml        manifest (required)
backend.wasm       WebAssembly component of the aphel:plugin world (optional)
web/index.js       ES module bundle that default-exports defineModule(...) (optional)
web/*              any other web asset (CSS, images)
```

`aphel-plugin pack` builds and zips everything (see [Tools](#tools)). Limits: 50 MB per package, 500 files, paths without `..`.

## Manifest (`plugin.toml`)

```toml
key = "equine"                 # lowercase, unique, not a built-in module key
name = "Haras"
version = "2.0.0"              # semver
description = "Stud farm"
api = 1                        # plugin API version
requires_aphel = ">=0.1.0"
requires = ["deals.read", "contacts.read"]   # core permissions the plugin uses (see Capabilities)
events = ["deal.won"]          # domain events the backend receives ("deal.*" works too)

[[entities]]                   # new record types: custom objects managed by the plugin
key = "horse"                  # record type co_equine_horse, object key equine_horse
label = "Cavalo"
label_plural = "Cavalos"
icon = "star"
owner_enabled = true

[[entities.fields]]
key = "sex"
label = "Sexo"
type = "select"                # any custom field type
required = true
options = [{ value = "stallion", label = "Garanhão" }, { value = "mare", label = "Égua" }]

[[entities.fields]]
key = "sire"
label = "Pai"
type = "record"
target = "co_equine_horse"     # record fields may point to any record type, including custom objects

[[fields]]                     # fields added to existing record types (managed custom fields)
record_type = "deal"
key = "horse"
label = "Cavalo negociado"
type = "record"
target = "co_equine_horse"

[[hooks]]                      # record.saving | record.validating | record.saved | record.deleting
hook = "record.validating"
record_types = ["deal", "co_equine_horse"]
priority = 25                  # order among plugins (lower first)

[[routes]]                     # served at /api/v1/p/<key><path>
method = "GET"
path = "/horses/{id}/pedigree"
permission = "co_equine_horse.read"   # a core, entity or declared permission; omitted = any signed-in user

[[routes]]                     # public route: /api/v1/public/p/<key><path>, no session
method = "POST"
path = "/webhook"
public = true

[[permissions]]                # permissions the plugin adds to the role editor
key = "equine.export_pedigree" # always "<plugin key>.<action>"
label = "Exportar genealogia"
description = "Baixar a genealogia completa em PDF"
grant_to = ["manager"]         # roles that receive it on the first install (admin always has everything)

[http]
allowed_hosts = ["api.abccmm.org.br", "*.example.org"]   # outbound HTTP allowlist (exact or *.domain, max 20)

[[schedules]]                  # at most 10; cron with seconds, like the platform schedules
name = "daily_sync"
cron = "0 0 3 * * *"

[[settings]]                   # edited by administrators, read by the plugin
key = "exam_validity_days"
label = "Validade dos exames (dias)"
type = "number"                # number | boolean | text | secret
default = 60
min = 1
max = 365

[[settings]]
key = "api_token"
label = "Token da associação"
type = "secret"                # stored encrypted, never returned by the API (no default)

[web]
entry = "web/index.js"
style = "web/style.css"        # optional
```

What the declarations become:

| Declaration | Effect |
| --- | --- |
| `entities` | A custom object `co_<key>_<entity>` with `module_key = <key>`. It appears in the menu, lists, detail pages, forms, filters, search, import/export, automations and webhooks like any custom object. Administrators cannot edit or delete it by hand (409 `custom_object_managed_by_module`). Its permissions are `co_<key>_<entity>.read\|create\|update\|delete\|archive\|export` |
| `fields` | Custom fields with `module_key = <key>` on existing types (`deal`, `contact`, …), locked like any managed field (409 `field_managed_by_module`) and written by every record API through `custom_fields` |
| `requires` | Core permissions (or `co_*` permissions) the plugin may use. Unknown permissions make the manifest invalid |
| `permissions` | Entries in the permission catalog and the role editor (grouped by plugin key), granted to `grant_to` on the first install, toggled with enable/disable and removed with their grants on uninstall. A key already used gives 409 `permission_already_registered` |
| `hooks` | The backend is called inside the record transaction. An error vetoes the write (422 with the plugin code and fields), and `record.saving` may change `values`/`custom_fields` |
| `events` | The worker calls the backend after the transaction, at least once and with retries, as the plugin (see Capabilities). A plugin never receives the events it caused |
| `routes` | `GET/POST/PUT/PATCH/DELETE /api/v1/p/<key>/…` with the caller's session or API token and the declared permission. Public routes (see below) |
| `http` | Outbound HTTP allowed only to these hosts (see Outbound HTTP) |
| `schedules` | The worker calls `on-schedule(name, slot)` once per due slot (see Schedules) |
| `settings` | Shown in the plugin settings dialog; `PUT /api/v1/plugins/<key>/settings` |

## Capabilities and scope

Every call the plugin makes runs with **the actor's scope ∩ the plugin's capabilities**. The capabilities are:
- `requires`,
- the declared `permissions`,
- the permissions of the plugin's own entities (`co_<key>_*`).

A plugin never gets more than it declared, and never more than the person behind the action.

| Entry point | Actor | Scope |
| --- | --- | --- |
| Hooks | Whoever writes the record (user, API token, automation) | Their permissions ∩ capabilities |
| Private routes | The caller (session or API token) | Their permissions ∩ capabilities |
| Events, schedules, public routes | The plugin itself (`Actor::Plugin(key)`, shown as "Plugin <key>" in the audit) | The installer's permissions ∩ capabilities (`through_plugin`) |

The installer is the administrator who installed the plugin; an upgrade makes the upgrading administrator the installer, because they give the consent again. If the installer is deactivated, events, schedules and public routes fail with 503 `plugin_owner_inactive` until someone reinstalls the plugin.

Referencing a record counts as reading it. Writing a `record` field (for example, a contact in `owner_contact`) is validated with the plugin's scope, so the plugin needs read access to the referenced type (`contacts.read`). Without it, the reference is reported as not found.

## Consent

`POST /api/v1/plugins?dry_run=true` with the package returns what the plugin declares and installs nothing:
- entities and extended record types;
- hooks, events and routes, including public routes;
- `requires` and declared permissions;
- allowed hosts, schedules and secrets;
- the currently installed version.

The web app shows this as a consent dialog, with warnings for public routes and outbound HTTP, before the real install.

## Backend in Rust

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib"]

[dependencies]
aphel-plugin-sdk = { git = "https://github.com/Viniciusadm/aphel", tag = "v0.1.0" } # or a path to sdk/rust/aphel-plugin-sdk
```

```rust
use aphel_plugin_sdk::{Ctx, Event, HookCall, HookOutcome, Plugin, PluginError, Request, Response, Result, export_plugin, hooks, json};

struct Equine;

impl Plugin for Equine {
    fn on_hook(&self, ctx: &Ctx, call: &HookCall) -> Result<HookOutcome> {
        if call.is(hooks::VALIDATING, "deal") && call.changed("status").is_some_and(|change| change.new == "won") {
            let deal = ctx.records().get("deal", &call.record_id)?;
            // read the horse with ctx.records().get("co_equine_horse", id) and veto:
            return Err(PluginError::new("equine_exam_expired", "exames vencidos"));
        }
        Ok(HookOutcome::default())
    }

    fn on_event(&self, ctx: &Ctx, event: &Event) -> Result<()> {
        // event.event_type == "deal.won": ctx.records().update("co_equine_horse", id, json!({ "custom_fields": { "status": "sold" } }))
        Ok(())
    }

    fn on_schedule(&self, ctx: &Ctx, name: &str, slot: &str) -> Result<()> {
        // name == "daily_sync": let token = ctx.secret("api_token")?; ctx.http().get("https://api.abccmm.org.br/...")
        Ok(())
    }

    fn on_route(&self, ctx: &Ctx, request: &Request) -> Result<Response> {
        if request.is("GET", "/horses/{id}/pedigree") {
            return Ok(Response::ok(json!({ "id": request.param("id") })));
        }
        Err(PluginError::not_found("route_not_found", "no such route"))
    }
}

export_plugin!(Equine);
```

`Ctx` is the plugin API (every call runs with the scope described in [Capabilities](#capabilities-and-scope)):

| Call | What it does |
| --- | --- |
| `ctx.records().get/list/create/update/delete(record_type, …)` | Any record type through its record handler: permissions, field permissions, validation, hooks, events and audit apply. `list` takes a `ListQuery` (filter AST, sort, search, pagination up to 100). Inside a hook only reads are allowed (see below) |
| `ctx.kv(collection).get/set/delete/list` | Private key-value storage of the plugin (`plugins_data`), JSON values, collection up to 64 characters, id up to 128 |
| `ctx.settings()`, `ctx.setting::<T>(key)` | Settings with defaults applied (secrets are left out) |
| `ctx.secret(name)` | A `secret` setting, or `None` when it is not configured; an undeclared name gives 404 `plugin_secret_not_declared` |
| `ctx.http().send(HttpRequest::new(method, url).header(..).json(&body)?)`, `ctx.http().get(url)` | Outbound HTTP (see below) |
| `ctx.now()`, `ctx.log(level, message)` / `ctx.info(..)` | Clock and server log |

**Writes inside hooks.** Hooks run inside the transaction of the record being written. Creating, updating or deleting records from a hook fails with 409 `plugin_write_in_hook`: a second transaction would not be atomic with the first and could deadlock on the same row. Do the write from an event instead. Reads inside a hook see the committed state, not the row being written; use the `values`/`changes` of the `HookCall` for the new values.

**Errors.** Plugin errors are built with:
- `PluginError::new(code, message)` (422);
- `PluginError::field(code, field, message)` (422 with `details.fields`);
- `not_found` (404) and `forbidden` (403);
- `with_status(409)` for any other status.

Codes are lowercase snake case. Errors from host calls keep their HTTP status when the plugin propagates them with `?` (for example 403 `permission_denied`, 409 `plugin_write_in_hook`).

**The actor.** Every `HookCall`, `Event` and `Request` carries an `actor`:
- `kind`: `user`, `api_token`, `automation`, `plugin`, `system` or `anonymous` (public routes).
- `id`: the API token, automation or plugin key.
- `user_id`: the person behind the action. In hooks and private routes it is the user, the creator of the API token or the publisher of the automation. In events it is only filled when `kind` is `user`, because an event records just the token or automation id.

**Events** also carry `occurred_at` (RFC 3339).

**Request headers.** `Request.headers` holds the request headers as lowercase name/value pairs, in order and with repeated headers included. `request.header(name)` returns the first one and `request.header_values(name)` all of them. `authorization`, `cookie` and `proxy-authorization` never reach a plugin.

**Outbound HTTP** only reaches the hosts in `[http] allowed_hosts`:
- It goes through the same SSRF guard as webhooks: `WEBHOOKS_ALLOW_PRIVATE_NETWORKS` and the organization's `require_https`.
- No redirects and no proxy.
- The timeout is the remaining time of the call; bodies are capped at 5 MB.
- Errors: 403 `plugin_host_not_allowed` (undeclared host), 422 `plugin_host_not_allowed` (invalid or private URL), 503 `plugin_http_failed`.

**Secrets** are `[[settings]]` of `type = "secret"`:
- Stored in the `SecretVault` (`owner_type = "plugin"`, `owner_id = <key>`).
- In `PUT /api/v1/plugins/<key>/settings`, a string sets the secret, `null` removes it and an absent key keeps it.
- `GET /api/v1/plugins` only reports `settings[].configured`, and the audit writes `[redacted]`.
- Removed on uninstall.

**Schedules.**
- Every minute, the job `plugins.tick` enqueues one `plugin.schedule` job per due slot of each enabled plugin. It looks back 5 minutes and caps at 60 slots per tick.
- The job's unique key is `plugin.schedule:<key>:<name>:<slot>`, so a slot runs once even with several workers.
- `slot` is the RFC 3339 fire time in UTC. Use it to make the work idempotent.

**Public routes** (`public = true`):
- Served at `ANY /api/v1/public/p/<key>/<path>`, with no session.
- Rate limited: 60 requests per minute per IP and plugin, and 600 per minute per plugin.
- The actor kind is `anonymous`, and the call runs with the plugin's scope.
- A public route never matches on the private path, and vice versa.

**Isolation.** Each call runs in a fresh WebAssembly instance:
- at most 64 MB of memory;
- a time limit of 2 s in hooks, 10 s in routes, 30 s in events and 60 s in schedules;
- no filesystem, sockets or SQL outside the plugin API.

Calls are made once per write, request, event or slot, never once per listed row.

**A backend that stops loading.** If an installed backend stops loading (for example, it was built against another contract):
- the plugin list shows `backend_error`;
- its events, schedules and routes are skipped;
- every write to a record type it hooks fails with 503 `plugin_failed` until the plugin is reinstalled.

A veto is never silently bypassed.

**Build.** Run `cargo build --release --target wasm32-wasip2` (after `rustup target add wasm32-wasip2`); the output is already a component.

### The contract (WIT)

The backend is a [WebAssembly component](https://component-model.bytecodealliance.org) of the world `aphel:plugin@1.0.0`:
- It is defined in [`back/modules/plugins/wit/plugin.wit`](../../back/modules/plugins/wit/plugin.wit) and shipped inside the SDK crate as `sdk/rust/aphel-plugin-sdk/wit/plugin.wit`. That copy is kept in sync by `make modules` and checked by `make lint`.
- It exports `on-hook`, `on-event`, `on-route` and `on-schedule`.
- It imports the `host` interface: `get-record`, `list-records`, `create-record`, `update-record`, `delete-record`, `kv-get`, `kv-set`, `kv-delete`, `kv-list`, `http-fetch`, `settings-json`, `secret`, `now` and `log`.
- Record values travel as JSON strings inside the typed records, because record fields are dynamic (custom fields, custom objects).

`aphel-plugin-sdk` wraps the bindings generated by `wit-bindgen` in the `Plugin` trait and `Ctx` above. Any language with component tooling (`wit-bindgen` for C, Go or C#; `componentize-js`; `componentize-py`) can implement the same world directly.

**Versioning.** `1.0.0` has not been released yet. From its first release on, `1.x` only accepts additions that do not change existing types: new host functions, new interfaces and new optional exports. The shape of an existing `record`, `enum` or function signature is frozen, because a component compiled against it no longer links if a field is added, removed or reordered. Such a change needs `aphel:plugin@2.0.0`. Serving two major versions at the same time is not implemented: the host binds a single world, so introducing `2.0.0` also means adding a second set of bindings to the host.

The standard library of `wasm32-wasip2` imports WASI interfaces. The host provides them with an empty sandbox (no filesystem, no environment, no stdio, no sockets), so only clocks and randomness work; outbound HTTP goes through `http-fetch`.

## Upgrades

Uploading a package with the key of an installed plugin upgrades it, after the same consent screen. The schema is reconciled without deleting data:
- New entities, fields and options are created. Changed labels, `required`, `searchable`, targets and option labels are updated.
- Fields, entities and select options that the new version no longer declares are **deactivated**: hidden, with their data kept. A later version that declares them again reactivates them. Enabling the plugin does not reactivate them.
- Changing the **type** of an existing field is refused with 409 `plugin_field_type_changed` before anything is applied. The values live in type-specific columns and are not converted, so declare a field with a new key instead.

## Web in React

The bundle is an ES module that default-exports a `defineModule` (same API as the built-in modules, see [`docs/modules/web.md`](../modules/web.md)). It imports the Aphel runtime by bare specifiers that the Aphel page maps with an import map, so there is a single React and a single router:

| Specifier | Content |
| --- | --- |
| `react`, `react/jsx-runtime`, `react-dom`, `react-router`, `@tanstack/react-query`, `react-i18next` | The instances used by Aphel |
| `@aphel/web-sdk` | Stable API: `defineModule`, `NAV_ORDER`, `useModuleTranslation`, `Slot`, `api`, `useSession`, `RecordListView`, `RecordDetailPage`, `RecordFormDialog`, `RecordForm`, `useRecord`, `registerRecordResource`, formatting helpers, `customObjectPath`/`customRecordPath`, feedback (`ConfirmDialog`, `notifySuccess`, `notifyError`) and UI components (`Button`, `Badge`, `Card`, `EmptyState`, `ErrorState`, `Skeleton`, `Input`, `Label`, `Tooltip`, `Avatar`, `cn`) — [`web/src/sdk/index.ts`](../../web/src/sdk/index.ts) |

```js
// vite.config.js
export default {
  build: {
    lib: { entry: 'src/index.tsx', formats: ['es'], fileName: () => 'index.js' },
    rolldownOptions: {
      external: ['react', 'react/jsx-runtime', 'react-dom', 'react-router', '@tanstack/react-query', 'react-i18next', '@aphel/web-sdk'],
    },
  },
}
```

```tsx
import { defineModule } from '@aphel/web-sdk'

export default defineModule({
  key: 'equine',                                   // the plugin key
  translations: { 'pt-BR': ptBR, en },             // objects (or loaders); read with useModuleTranslation('equine')
  register(app) {
    app.recordTab('co_equine_horse', { key: 'equine.pedigree', label: 'pedigree.title', render: PedigreeTab })
    app.recordTab('contact', { key: 'equine.horses', label: 'contactTab', render: ContactHorsesTab })
    app.slot('record.detail.header', { recordTypes: ['co_equine_horse'], render: ExamBadges })
  },
})
```

Other libraries (icons, date helpers) are bundled into the plugin. Use `useModuleTranslation(key)` instead of `useTranslation` (the typed namespaces only know the built-in modules). Tailwind classes already used by Aphel work as is; anything else goes in the plugin `style.css`.

For types, point `@aphel/web-sdk` and `@/*` to an Aphel checkout in the plugin `tsconfig.json`:

```json
"paths": {
  "@aphel/web-sdk": ["../aphel/web/src/sdk/index.ts"],
  "@/*": ["../aphel/web/src/*"],
  "*": ["../aphel/web/node_modules/@types/*", "../aphel/web/node_modules/*"]
}
```

The web app loads `GET /api/v1/public/plugins` before it builds its routes, and imports each enabled bundle from `/api/v1/public/plugins/<key>/web/…` (public, cached by version). Installing, enabling or disabling a plugin reloads the page.

## Tools

`sdk/cli` builds the `aphel-plugin` command (`cargo build --release` in `sdk/cli`):

| Command | What it does |
| --- | --- |
| `aphel-plugin new <key> --sdk <path to aphel-plugin-sdk>` | Creates `aphel-<key>/` with `plugin.toml`, the crate and a `/ping` route |
| `aphel-plugin build [dir]` | `cargo build --release --target wasm32-wasip2` → `backend.wasm`, and `npm run build` in `web/` when it has a `package.json` |
| `aphel-plugin pack [dir] [-o file]` | Builds and zips `plugin.toml`, `backend.wasm` and `web/dist/*` (as `web/*`) into `<key>-<version>.aphelplugin` |

Install from the command line with an API token that has `system.manage`:

```bash
curl -H "Authorization: Bearer $TOKEN" -F package=@equine-2.0.0.aphelplugin "https://crm.example.com/api/v1/plugins?dry_run=true"   # what it asks for
curl -H "Authorization: Bearer $TOKEN" -F package=@equine-2.0.0.aphelplugin https://crm.example.com/api/v1/plugins                # install
```

## Administration API

| Route | Permission | |
| --- | --- | --- |
| `GET /api/v1/plugins` | `system.manage` | Installed plugins, what each declares, `backend_error` and `settings[].configured` |
| `POST /api/v1/plugins` (multipart `package`) | `system.manage` | Install or upgrade; `?dry_run=true` only returns the consent payload |
| `POST /api/v1/plugins/{key}/enable` · `/disable` | `system.manage` | Disabling hides entities, fields and permissions and stops hooks, events, schedules, routes and the web bundle; nothing is deleted |
| `PUT /api/v1/plugins/{key}/settings` | `system.manage` | Validated against the declared settings; secrets set, removed (`null`) or kept (absent) |
| `DELETE /api/v1/plugins/{key}?purge=true` | `system.manage` | Removes the plugin, its key-value data, secrets and permissions; with `purge` also its entities, records and fields |
| `GET /api/v1/public/plugins` | public | Web bundles of the enabled plugins |
| `ANY /api/v1/p/{key}/{path}` | declared per route | Plugin routes |
| `ANY /api/v1/public/p/{key}/{path}` | public, rate limited | Public plugin routes |

Every change publishes `plugin.installed|enabled|disabled|settings_updated|uninstalled` with audit.

## How it works

The compiled module `plugins` ([`back/modules/plugins`](../../back/modules/plugins)) is the host:

- **Storage.** The tables are `plugins_installed` (with `installed_by`), `plugins_files` (package files), `plugins_data` (key-value) and `plugins_state.version`. Declared permissions live in the kernel table `runtime_permissions` (owner `plugin:<key>`). The version is bumped on every change, and every `serve` and `worker` process rechecks it at most every 2 s. Only the plugins whose checksum changed are recompiled.
- **Runtime.** [`wasmtime`](https://wasmtime.dev) with the Component Model, Cranelift, epoch interruption and per-instance memory limits.
  - Host bindings come from `wasmtime::component::bindgen!` over the WIT file.
  - Each plugin is compiled once into a `PluginPre` (imports already resolved), so a call only instantiates and runs.
  - WASI comes from `wasmtime-wasi` with an empty sandbox.
- **Hooks.** The host registers one handler per record hook (`RecordSaving`, `RecordValidating`, `RecordSaved`, `RecordDeleting`). It forwards to the enabled plugins that subscribed to that hook and record type, flagging the call as in-transaction.
- **Events.** The consumer `plugins.events` accepts the event types that some enabled plugin consumes. It skips events whose actor is the plugin itself.
- **Schedules.** The jobs `plugins.tick` and `plugin.schedule` live in `schedule.rs`.
- **Routes.** Two generic routes dispatch `/api/v1/p/{key}/…` and `/api/v1/public/p/{key}/…` to the plugin, after matching the declared route and checking its permission or rate limit.
- **Schema.** Entities are created through the custom object use cases and locked with `custom_objects.module_key`. Fields are written to `field_definitions` with `module_key`, and the boot sync only releases fields of compiled modules. `schema.rs` reconciles upgrades as described in [Upgrades](#upgrades).
