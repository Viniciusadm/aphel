# Runtime plugins

A runtime plugin is a `.aphelplugin` package that an administrator uploads in **Settings › Integrations › Modules › Plugins** (or `POST /api/v1/plugins`). Installing, upgrading, enabling, disabling and uninstalling a plugin needs **no rebuild, no restart and no change to the Aphel repository**: the plugin lives in its own repository, and the running server (and every worker) picks it up within a few seconds.

| | Compiled modules ([`docs/modules`](../modules/README.md)) | Runtime plugins (this guide) |
| --- | --- | --- |
| Who | Aphel core and official modules | Third parties |
| Install | Line in `back/aphel.modules.toml` + rebuild | Upload of a `.aphelplugin` file |
| Backend | Rust with full access (`Ctx`, SQL, every hook) | Rust compiled to WebAssembly, sandboxed, through the plugin API |
| Data | Own tables and migrations | Entities (managed custom objects), managed fields and key-value storage |
| Web | Built with the Aphel web app | Prebuilt ES module loaded at runtime through an import map |

Reference plugin: `aphel-equine`, kept in its own repository next to Aphel (stud farm: horses with pedigree and sanitary exams, a field on deals, a rule that blocks winning a deal with expired exams and a reaction that transfers the horse when the deal is won).

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
permission = "co_equine_horse.read"   # any existing permission; omitted = any signed-in user

[[settings]]                   # edited by administrators, read by the plugin
key = "exam_validity_days"
label = "Validade dos exames (dias)"
type = "number"                # number | boolean | text
default = 60
min = 1
max = 365

[web]
entry = "web/index.js"
style = "web/style.css"        # optional
```

What the declarations become:

| Declaration | Effect |
| --- | --- |
| `entities` | A custom object `co_<key>_<entity>` with `module_key = <key>`: it appears in the menu, lists, detail pages, forms, filters, search, import/export, automations and webhooks like any custom object, and administrators cannot edit or delete it by hand (409 `custom_object_managed_by_module`). Its permissions are `co_<key>_<entity>.read|create|update|delete|archive|export` |
| `fields` | Custom fields with `module_key = <key>` on existing types (`deal`, `contact`, …), locked like any managed field (409 `field_managed_by_module`) and written by every record API through `custom_fields` |
| `hooks` | The backend is called inside the record transaction; an error vetoes the write (422 with the plugin code and fields), `record.saving` may change `values`/`custom_fields` |
| `events` | The backend is called by the worker after the transaction (at-least-once, with retries) with the system scope |
| `routes` | `GET/POST/PUT/PATCH/DELETE /api/v1/p/<key>/…` with the caller's session or API token and the declared permission |
| `settings` | Shown in the plugin settings; `PUT /api/v1/plugins/<key>/settings` |

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

    fn on_route(&self, ctx: &Ctx, request: &Request) -> Result<Response> {
        if request.is("GET", "/horses/{id}/pedigree") {
            return Ok(Response::ok(json!({ "id": request.param("id") })));
        }
        Err(PluginError::not_found("route_not_found", "no such route"))
    }
}

export_plugin!(Equine);
```

`Ctx` is the plugin API (all calls run with the scope of whoever acts: the user of the request or of the write, or the system for events):

| Call | What it does |
| --- | --- |
| `ctx.records().get/list/create/update/delete(record_type, …)` | Any record type through its record handler: permissions, field permissions, validation, hooks, events and audit apply. `list` takes a `ListQuery` (filter AST, sort, search, pagination up to 100) |
| `ctx.kv(collection).get/set/delete/list` | Private key-value storage of the plugin (`plugins_data`), JSON values |
| `ctx.settings()`, `ctx.setting::<T>(key)` | Settings with defaults applied |
| `ctx.now()`, `ctx.log(level, message)` / `ctx.info(..)` | Clock and server log |

Errors: `PluginError::new(code, message)` (422), `PluginError::field(code, field, message)` (422 with `details.fields`), `not_found` (404), `forbidden` (403), `with_status(409)`. Codes are lowercase snake case.

Every `HookCall`, `Event` and `Request` carries an `actor`: `kind` (`user`, `api_token`, `automation` or `system`), `id` (the API token or automation id) and `user_id` (the person behind the action, when there is one). Events also carry `occurred_at` (RFC 3339). `Request.headers` holds the request headers in lowercase (`request.header(name)`), except `authorization`, `cookie` and `proxy-authorization`, which never reach a plugin.

Isolation: each call runs in a fresh WebAssembly instance with at most 64 MB of memory and a time limit (2 s in hooks, 10 s in routes, 30 s in events). No filesystem, network or SQL access exists outside the plugin API. Calls are made once per write, request or event (never once per listed row).

Build with `cargo build --release --target wasm32-wasip2` (`rustup target add wasm32-wasip2`): the output is already a component.

### The contract (WIT)

The backend is a [WebAssembly component](https://component-model.bytecodealliance.org) of the world `aphel:plugin@1.0.0`, defined in [`back/modules/plugins/wit/plugin.wit`](../../back/modules/plugins/wit/plugin.wit) and shipped inside the SDK crate as `sdk/rust/aphel-plugin-sdk/wit/plugin.wit` (a copy kept in sync by `make modules` and checked by `make lint`). It exports `on-hook`, `on-event` and `on-route` and imports the `host` interface (`get-record`, `list-records`, `create-record`, `update-record`, `delete-record`, `kv-get`, `kv-set`, `kv-delete`, `kv-list`, `settings-json`, `now`, `log`). Record values travel as JSON strings inside the typed records, because record fields are dynamic (custom fields, custom objects). `aphel-plugin-sdk` wraps the bindings generated by `wit-bindgen` in the `Plugin` trait and `Ctx` above; any language with component tooling (`wit-bindgen` for C/Go/C#, `componentize-js`, `componentize-py`) can implement the same world directly. The world is versioned. Inside `1.x` only additions that do not change existing types are allowed: new host functions, new interfaces and new optional exports. The shape of an existing `record`, `enum` or function signature is frozen, because a component compiled against it no longer links if a field is added, removed or reordered; such a change becomes `aphel:plugin@2.0.0`, served side by side by the host.

The standard library of `wasm32-wasip2` imports WASI interfaces; the host provides them with an empty sandbox (no filesystem, no environment, no stdio, no sockets), so only clocks and randomness work.

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

For types, point `@aphel/web-sdk` and `@/*` to an Aphel checkout in the plugin `tsconfig.json` (`"paths": { "@aphel/web-sdk": ["../aphel/web/src/sdk/index.ts"], "@/*": ["../aphel/web/src/*"], "*": ["../aphel/web/node_modules/@types/*", "../aphel/web/node_modules/*"] }`).

The web app loads `GET /api/v1/public/plugins` before it builds its routes and imports each enabled bundle (`/api/v1/public/plugins/<key>/web/…`, public, cached by version). Installing, enabling or disabling a plugin reloads the page.

## Tools

`sdk/cli` builds the `aphel-plugin` command (`cargo build --release` in `sdk/cli`):

| Command | What it does |
| --- | --- |
| `aphel-plugin new <key> --sdk <path to aphel-plugin-sdk>` | Creates `aphel-<key>/` with `plugin.toml`, the crate and a `/ping` route |
| `aphel-plugin build [dir]` | `cargo build --release --target wasm32-wasip2` → `backend.wasm`, and `npm run build` in `web/` when it has a `package.json` |
| `aphel-plugin pack [dir] [-o file]` | Builds and zips `plugin.toml`, `backend.wasm` and `web/dist/*` (as `web/*`) into `<key>-<version>.aphelplugin` |

Install from the command line with an API token that has `system.manage`:

```bash
curl -H "Authorization: Bearer $TOKEN" -F package=@equine-2.0.0.aphelplugin https://crm.example.com/api/v1/plugins
```

## Administration API

| Route | Permission | |
| --- | --- | --- |
| `GET /api/v1/plugins` | `system.manage` | Installed plugins and what each declares |
| `POST /api/v1/plugins` (multipart `package`) | `system.manage` | Install or upgrade (upgrade reconciles entities and fields without deleting data) |
| `POST /api/v1/plugins/{key}/enable` · `/disable` | `system.manage` | Disabling hides entities and fields and stops hooks, events, routes and the web bundle; nothing is deleted |
| `PUT /api/v1/plugins/{key}/settings` | `system.manage` | Validated against the declared settings |
| `DELETE /api/v1/plugins/{key}?purge=true` | `system.manage` | Removes the plugin; with `purge` also its entities, records and fields |
| `GET /api/v1/public/plugins` | public | Web bundles of the enabled plugins |
| `ANY /api/v1/p/{key}/{path}` | declared per route | Plugin routes |

Every change publishes `plugin.installed|enabled|disabled|settings_updated|uninstalled` with audit.

## How it works

The compiled module `plugins` ([`back/modules/plugins`](../../back/modules/plugins)) is the host:

- **Storage:** `plugins_installed`, `plugins_files` (package files), `plugins_data` (key-value) and `plugins_state.version`. The version is bumped on every change; every `serve` and `worker` process rechecks it at most every 2 s and recompiles only the plugins whose checksum changed.
- **Runtime:** [`wasmtime`](https://wasmtime.dev) with the Component Model, Cranelift, epoch interruption and per-instance memory limits. Host bindings come from `wasmtime::component::bindgen!` over the WIT file; each plugin is compiled once into a `PluginPre` (imports already resolved), so a call only instantiates and runs. WASI comes from `wasmtime-wasi` with an empty sandbox.
- **Hooks:** the host registers one handler for each supported record hook (`RecordSaving`, `RecordValidating`, `RecordSaved`, `RecordDeleting`) and forwards to the enabled plugins that subscribed to that hook and record type.
- **Events:** the consumer `plugins.events` accepts the event types some enabled plugin consumes.
- **Routes:** one generic route dispatches `/api/v1/p/{key}/…` to the plugin after matching the declared route and checking its permission.
- **Schema:** entities are created through the custom object use cases and locked with `custom_objects.module_key`. Fields are written to `field_definitions` with `module_key`, and the boot sync only releases fields of compiled modules.
