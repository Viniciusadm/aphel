# Runtime plugins

A runtime plugin is a `.aphelplugin` package that an administrator uploads in **Settings › Integrations › Modules › Plugins** (or `POST /api/v1/wasm-runtime`). Installing, upgrading, enabling, disabling and uninstalling a plugin needs **no rebuild, no restart and no change to the Aphel repository**: the plugin lives in its own repository, and the running server (and every worker) picks it up within a few seconds.

| | Compiled modules ([`docs/modules`](../modules/README.md)) | Runtime plugins (this guide) |
| --- | --- | --- |
| Who | Aphel core and official modules | Third parties |
| Install | Line in `back/aphel.modules.toml` + rebuild | Upload of a `.aphelplugin` file, after a consent screen |
| Backend | Rust with full access (`Ctx`, SQL, every hook) | A WebAssembly component (Rust or JavaScript/TypeScript through the SDKs, or any language with component tooling), sandboxed, restricted to the permissions it declares |
| Data | Own tables and migrations | Entities (managed custom objects), managed fields, key-value storage and secrets |
| Web | Built with the Aphel web app | Prebuilt ES module loaded at runtime through an import map |

Reference plugins, kept in their own repositories next to Aphel:
- `aphel-equine`, a stud farm. It adds horses with pedigree and sanitary exams, plus a field on deals. A rule blocks winning a deal with expired exams, and a reaction transfers the horse when the deal is won.
- `aphel-probe`, which exercises every host call and hook kind. It is used as an end-to-end check of the platform.
- `aphel-hello-js`, the same contract written in JavaScript: hooks, routes, events, an automation action and a widget.

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
requires = ["deals.read", "people.read"]   # core permissions the plugin uses (see Capabilities)
events = ["deal.won"]          # domain events the backend receives ("deal.*" works too)

[[entities]]                   # new record types: custom objects managed by the plugin
key = "horse"                  # record type equine.horse, object key equine_horse
label = "Cavalo"
label_plural = "Cavalos"
icon = "star"
owner_enabled = true
permission_scope = "organization"   # optional: scope of the default role grants (own | team | organization)

[[entities.fields]]
key = "sex"
label = "Sexo"
type = "select"                # any custom field type
required = true
options = [{ value = "stallion", label = "Garanhão" }, { value = "mare", label = "Égua" }]

[[entities.fields]]
key = "registry"
label = "Registro"
type = "text"
unique = true                  # enforced by the database (text, number, url, email, phone)

[[entities.fields]]
key = "sire"
label = "Pai"
type = "record"
target = "equine.horse"     # record fields may point to any record type, including custom objects

[[fields]]                     # fields added to existing record types (managed custom fields)
record_type = "deal"
key = "horse"
label = "Cavalo negociado"
type = "record"
target = "equine.horse"

[[hooks.subscribe]]                      # record.* or a domain hook such as deal.winning (see the host catalog)
hook = "record.validating"
record_types = ["deal", "equine.horse"]
priority = 25                  # order among plugins (lower first)

[[hooks.declare]]              # a hook this plugin offers to others (key starts with "<plugin key>.")
key = "equine.horse_sold"
kind = "action"                # veto | filter | action, default action
description = "A horse was sold"
record_type = "equine.horse"   # optional, default "*"

operations = ["notes.create", "tagging.apply"]   # module operations called with invoke

[[routes]]                     # served at /api/v1/p/<key><path>
method = "GET"
path = "/horses/{id}/pedigree"
permission = "equine.horse.read"   # a core, entity or declared permission; omitted = any signed-in user

[[routes]]                     # public route: /api/v1/public/p/<key><path>, no session
method = "POST"
path = "/webhook"
public = true

[[permissions]]                # permissions the plugin adds to the role editor
key = "equine.export_pedigree" # always "<plugin key>.<action>"
label = "Exportar genealogia"
description = "Baixar a genealogia completa em PDF"
grant_to = ["manager"]         # roles that receive it on install, upgrade and "reapply grants" (admin always has everything)

[[role_presets]]               # roles the installer may create from the consent dialog (never created without consent)
key = "groom"                  # role key; also receives grants on upgrade when it exists
label = "Tratador"
description = "Cuida dos cavalos"
permissions = ["equine.horse.read", "equine.horse.update", "equine.export_pedigree"]
scope = "organization"         # own | team | organization (default organization)

[http]
allowed_hosts = ["api.abccmm.org.br", "*.example.org"]   # outbound HTTP allowlist (exact or *.domain, max 20)

[[schedules]]                  # at most 10; cron with seconds, like the platform schedules
name = "daily_sync"
cron = "0 0 3 * * *"

[[settings]]                   # edited by administrators, read by the plugin
key = "exam_validity_days"
label = "Validade dos exames (dias)"
type = "number"                # number | boolean | text | select | secret
group = "Exams"                # optional heading in the settings form
default = 60
min = 1
max = 365

[[settings]]
key = "reminder_channel"
label = "Reminder channel"
type = "select"                # needs options; the value is one of them
options = [{ value = "email", label = "Email" }, { value = "push", label = "Notification" }]
default = "email"

[[settings]]
key = "api_token"
label = "Token da associação"
type = "secret"                # stored encrypted, never returned by the API (no default)

[[actions]]                    # automation actions "plugin.<key>.<action>"
key = "request_exam"
label = "Solicitar exame"
description = "Pede um exame ao laboratório"
record_types = ["equine.horse"]   # record types the action runs on (empty = any)
retry_safe = true              # the automation may retry it after a failure
inputs = [
  { key = "exam", label = "Exame", type = "select", options = ["aie", "mormo"], required = true },
  { key = "note", label = "Observação", type = "text" },   # text | number | boolean | select
]

[[emits]]                      # events the plugin publishes: "plugin.<key>.<name>", usable as automation triggers
name = "exam_expired"
label = "Exame vencido"
record_types = ["equine.horse"]

[[notifications]]              # notification kinds "<key>_<name>" in every user's preferences
name = "exam_expiring"
label = "Exame vencendo"
description = "Um exame do cavalo vence em breve"
channels = ["in_app", "email"] # in_app | email | push
email_default = false
push_default = false

[[widgets]]                    # dashboard widgets "<key>_<name>"
name = "horses_for_sale"
title = "Cavalos à venda"
category = "equine"            # optional, default "plugins"
permission = "equine.horse.read"     # optional: the viewer needs it
size = "medium"                # small | medium | large
uses_period = false            # default true: the dashboard period is passed
default_settings = { limit = 5 }

[[provides.operations]]        # operations other plugins call with ctx.call and invoke
key = "equine.estimate_value"  # dotted lowercase, unique, never a native key
description = "Estimates the value of a horse"
effect = "read"                # read | write | external, default read
permission = "equine.horse.read"   # optional: the caller needs it
idempotent = true              # optional

[[contribute]]                 # a typed contribution to an extension point
point = "dashboard.widget"
name = "horses_for_sale"
title = "Cavalos à venda"      # optional
data = {}                      # optional, validated by the point

[web]
entry = "web/index.js"
style = "web/style.css"        # optional
```

What the declarations become:

| Declaration | Effect |
| --- | --- |
| `entities` | A custom object `custom.<key>_<entity>` with `module_key = <key>`. It appears in the menu, lists, detail pages, forms, filters, search, import/export, automations and webhooks like any custom object. Administrators cannot edit or delete it by hand (409 `custom_object_managed_by_module`). Its permissions are `<key>.<entity>.read\|create\|update\|delete\|archive\|export` (for example `equine.horse.read`); they belong to the plugin's capabilities without being listed in `requires` |
| `fields` | Custom fields with `module_key = <key>` on existing types (`deal`, `contact`, …), locked like any managed field (409 `field_managed_by_module`) and written by every record API through `custom_fields` |
| `requires` | Core permissions (or `custom.*` permissions) the plugin may use. Unknown permissions make the manifest invalid |
| `permissions` | Entries in the permission catalog and the role editor (grouped by plugin key), granted with `organization` scope to the `grant_to` roles that exist on install, upgrade and "reapply grants", toggled with enable/disable and removed with their grants on uninstall. A key already used gives 409 `permission_already_registered` |
| `entities[].permission_scope` | Scope of the default grants of the entity permissions to the system `manager` and `member` roles (otherwise `team` for managers and `own` for members). Shared operational records use `organization`; personal ones keep `own`. Existing grants are never narrowed or widened, only missing ones are added |
| `entities[].fields[].unique` | Database-enforced uniqueness (case-insensitive for text) on text, number, url, email and phone fields: a reservation row per value in `custom_field_unique_values`, written in the same transaction as the value, released when the record is deleted and taken again (or refused) when it is restored. A concurrent duplicate fails with 422 and a `custom_fields.<key>` violation, whatever API writes it. Declaring `unique` on a field that already has duplicates is refused with 409 `plugin_field_unique_conflict` (with the duplicated record ids in `details.conflicts`) before anything is written, also in the dry run; removing `unique` keeps the data and drops the reservations |
| `entities[].internal` | `true` hides the entity from the places that list record types for people (global search, role matrix, type selectors). Its permissions keep working |
| `role_presets[].home` | Application path where users of the role land when they have not picked a home in their navigation settings |
| `role_presets` | Roles the plugin suggests, with permissions taken from its declared permissions, its `requires` and its entity permissions. The consent dialog lists them unchecked; only the selected ones are created (multipart field `role_presets`, comma separated). Presets whose role already exists receive the missing grants |
| `optional_requires` | Permissions the plugin uses when their module is enabled. Unlike `requires`, a disabled module does not block the install; check with `ctx.can` before using them |
| `hooks` | Generic record hooks or domain hooks of the modules (see [Module operations and domain hooks](#module-operations-and-domain-hooks)). The backend is called inside the transaction. An error from a veto hook blocks the write (422 with the plugin code and fields); `record.saving` and the filter hooks may change the values |
| `operations` | Module operations the backend calls with `invoke` (see [Module operations and domain hooks](#module-operations-and-domain-hooks)). An operation not listed gives 403 `plugin_operation_not_declared`; an unknown one makes the manifest invalid |
| `events` | The worker calls the backend after the transaction, at least once and with retries, as the plugin (see Capabilities). A plugin never receives the events it caused |
| `routes` | `GET/POST/PUT/PATCH/DELETE /api/v1/p/<key>/…` with the caller's session or API token and the declared permission. Public routes (see below) |
| `http` | Outbound HTTP allowed only to these hosts (see Outbound HTTP) |
| `schedules` | The worker calls `on-schedule(name, slot)` once per due slot (see Schedules) |
| `settings` | Shown in the plugin settings dialog; `GET`/`PUT /api/v1/wasm-runtime/<key>/settings`. `type = "select"` takes `options` (`value` and `label`), `group` sorts settings under a heading. `settings_permission` (a permission the plugin declares) lets people who hold it read and write the settings without `system.manage`, so the plugin can offer them in its own screens |
| `actions` | Automation actions `plugin.<key>.<action>` in the automation editor, with a form built from `inputs` (see Automations) |
| `emits` | Events `plugin.<key>.<name>` the backend publishes with `emit`, available as automation triggers |
| `notifications` | Notification kinds `<key>_<name>` in the preferences, sent with `notify` (see Notifications) |
| `widgets` | Dashboard widgets `<key>_<name>` whose data comes from `on-widget` (see Dashboard widgets) |
| `uses.required` / `uses.optional` | Contract requirements `name@major`. On install, upgrade and enable every required contract needs an enabled provider (a native module or another enabled plugin with it in `provides.contracts`, same major); otherwise 422 `plugin_contract_unavailable` lists the missing ones. Optional contracts are never checked. Upgrading, disabling or uninstalling a provider is refused with 409 `plugin_has_dependents` (naming the plugins) when an enabled plugin would lose a required contract it had, or an operation it lists in `operations` |
| `hooks.declare` | Hooks the plugin offers. While enabled they are in the host catalog, so other plugins subscribe with `[[hooks.subscribe]]`. Firing a declared hook from the backend is not available yet |
| `provides.operations` | Operations the plugin offers. While the plugin is enabled each one is in the operation registry (`ctx.call`, `invoke`, the platform catalog, `aphel catalog`); the host runs `on-operation(key, input-json)` with the caller's scope intersected with the plugin's capabilities. A key owned by a native module is refused on install and upgrade unless the plugin holds that module through `provides.contracts` (it then takes over the key while enabled); a key owned by another installed plugin is refused; a read operation runs read-only. Disabling or uninstalling the plugin removes them |
| `contribute` | Contributions to an extension point. They are validated on install and upgrade and, while the plugin is enabled, registered as `RuntimeContribution` entries that the point's owner reads with `App::runtime_contributions(point)`; calling one routes to `on-contribution(point, name, input-json)` with the restricted scope. The point's own schema is not checked yet |
| `privacy.subject_data` | The plugin holds personal data outside its record fields and takes part in privacy requests (see Data subject requests) |
| `web.public` | Folder under `web/` served as a public single-page app at `/p/<key>/` (see Public web app) |

Record types in manifests and in `ctx.records()` calls use the canonical names listed by `GET /api/v1/entities` (`people.person`, `organizations.organization`, `deals.deal`, `loyalty.account`). A field `target` with an alias such as `contact` is refused with `record_type_not_found`. In a `list` filter, an entity's own fields are addressed by their key (`contact`), not by `custom_fields.contact`; filtering on an unknown field answers 422 `filter.field is not a known field`.

An entity can opt into platform behaviours with `traits` in its `[[entities]]` table, for example `traits = { "privacy.personal_data" = true, "party.contact_points" = { points = [ { field = "email", kind = "email" }, { field = "phone", kind = "phone" } ] } }`. `party.contact_points` maps the fields that hold e-mail, phone and web addresses; `privacy.personal_data` together with `personal = true` on those fields makes privacy subject searches find the records. `aphel catalog` lists every entity trait in use and every contribution point, with the modules that contribute to it. `bulk_fields = ["owner_id", "stage"]` in `[[entities]]` turns on the bulk action bar (assign owner, tags, set field, archive, delete, export) for the entity, limited to those core fields plus any custom field; `workflow.staged` gives the Stages board and every entity appears as a record type in the automation triggers.

Entity field keys cannot reuse the keys every custom object already has: `name`, `title`, `owner_id`, `created_by`, `created_at` and `updated_at`. Any key conflict (an object or field that already exists and is not the plugin's) is refused with 409 `plugin_object_conflict` or `plugin_field_conflict` before anything is written.

### Menu and replaced modules

A plugin integrates with the navigation through `[menu]` and replaces a built-in module by providing its contract:

```toml
[provides]
contracts = ["proposals@1"]

[menu]
flat = true
own_entities = "hidden"
```

| Field | Meaning |
| --- | --- |
| `menu.flat` | The plugin's `app.nav` items ignore their `group`: they appear as regular items, not as a submenu named after the plugin |
| `menu.own_entities` | `hidden` removes the plugin's entities (custom objects with its `module_key`) from the sidebar; default `visible` |
| `conflicts_with`, `menu.rules` | Removed. A manifest that still has them is rejected (`plugin_manifest_invalid`) with a message naming the replacement |
| `provides.contracts` | Contracts the plugin provides (`<name>@<major>`). A contract has one provider: when a built-in module declares the same contract, installing (or enabling) the plugin disables that module and uninstalling (or disabling) it enables it again, only if it was enabled before and no other plugin still holds it. While held, enabling the module is refused (409 `module_held_by_extension`). The navigation of a held module disappears with it; blueprints own the order of the remaining sections |

Conflicts between plugins are detected on install, upgrade and enable: hooks on the same `(hook, record_type)` for `record.saving|validating|deleting|archiving` of a type neither owns, or the same role preset key. They need `acknowledge_conflicts=true` (409 `plugin_conflicts_unacknowledged`).

## Capabilities and scope

Every call the plugin makes runs with **the actor's scope ∩ the plugin's capabilities**. The capabilities are:
- `requires`,
- the declared `permissions`,
- the permissions of the plugin's own entities (`<key>.<entity>.*`).

A plugin never gets more than it declared, and never more than the person behind the action.

| Entry point | Actor | Scope |
| --- | --- | --- |
| Hooks | Whoever writes the record (user, API token, automation) | Their permissions ∩ capabilities |
| Private routes | The caller (session or API token) | Their permissions ∩ capabilities |
| Automation actions | The automation (`Actor::Automation`) | The permissions of whoever published the automation ∩ capabilities |
| Widgets | The viewer | Their permissions ∩ capabilities |
| Events, schedules, public routes | The plugin itself (`Actor::Plugin(key)`, shown as "Plugin <key>" in the audit) | The installer's permissions ∩ capabilities (`through_plugin`) |

The installer is the administrator who installed the plugin; an upgrade makes the upgrading administrator the installer, because they give the consent again. If the installer is deactivated, events, schedules and public routes fail with 503 `plugin_owner_inactive` until someone reinstalls the plugin.

Referencing a record counts as reading it. Writing a `record` field (for example, a contact in `owner_contact`) is validated with the plugin's scope, so the plugin needs read access to the referenced type (`people.read`). Without it, the reference is reported as not found.

## Consent

`POST /api/v1/wasm-runtime?dry_run=true` with the package returns what the plugin declares and installs nothing:
- entities and extended record types;
- hooks, events and routes, including public routes;
- the module operations it calls;
- `requires` and declared permissions;
- role presets, entity visibility (`permission_scope`) and unique fields;
- allowed hosts, schedules and secrets;
- `menu` and the contracts it provides (`provides.contracts`);
- `conflicts` with other plugins and `module_conflicts` (modules that will be disabled);
- the currently installed version;
- `grants`: for every role the plugin grants to, whether it exists and how many of the expected permissions it already has.

The web app shows this as a consent dialog, with warnings for public routes and outbound HTTP and an unchecked checklist of role presets, before the real install.

Installing never claims the setup is complete: `GET /api/v1/wasm-runtime` returns `grants` (per role: `role_exists`, `expected`, `granted`, `missing`) and `ungranted_roles`, and the plugin screen shows "role: 14 of 18 permissions granted". After creating a missing role in Settings › Roles, `POST /api/v1/wasm-runtime/{key}/grants` reapplies the declared grants to the roles that exist now (it never creates roles). Install and reapply publish `plugin.installed` / `plugin.grants_reapplied` with the created roles and the number of grants in the audit, in the same transaction as the permission writes.

Plugin entities are managed custom objects: they are created with `module_key` already set and do not count against the 50 custom objects an organization can create.

## Module operations and domain hooks

Besides records, a plugin reaches the modules through two generic channels. Both are open catalogs: each module contributes its own entries, so the WIT does not change when a module offers something new.

**Operations.** `invoke(operation, input-json)` calls a use case of a module (`notes.create`, `tagging.apply`, `deals.move_stage`, `email.send`, `sequences.enroll`, `ai.complete`, …). The operation runs the same service as the module's API, with the plugin's scope (see [Capabilities](#capabilities-and-scope)): permissions, events and audit apply, and the plugin needs the matching permissions in `requires` (`notes.create` needs `notes.create`). The plugin lists the operations it calls in `operations`, and the installer sees them in the consent. Operations provided by another plugin (`provides.operations`) are called the same way once that plugin is enabled; the provider's `permission` is checked against the original actor's scope (a plugin that calls another plugin's operation does not need the provider's permissions itself, and the provider runs with the actor's scope intersected with its own capabilities), a plugin cannot call itself through a cycle (the execution chain refuses it), and writing operations are refused inside hooks.

| Situation | Result |
| --- | --- |
| Operation not in `operations` | 403 `plugin_operation_not_declared` |
| Operation unknown or its module disabled | 404 `plugin_operation_unknown` |
| Writing operation inside a hook | 409 `plugin_write_in_hook` |
| Operation slower than the remaining call time | 503 `plugin_operation_timeout` |

Operations that act on the user's own data (favorites, reminders, following) need a user actor: they work from hooks and private routes and are refused from events and schedules, where the actor is the plugin.

**Domain hooks.** Besides the `record.*` hooks, modules offer hooks for their own moments (`deal.winning`, `deal.won`, `lead.converting`, `quote.accepting`, `activity.completing`, `email.outgoing`, …). The plugin subscribes to them in `[[hooks.subscribe]]` like the record hooks (`record_types` is the type in the catalog, or `*`). The `HookCall` carries the hook key, the record and the domain data in `values`:

| Kind | Behaviour |
| --- | --- |
| `veto` | An error blocks the operation (the deal is not won, the quote is not accepted) |
| `filter` | The returned `values` is merged into the data (for example the deal and company maps of a lead conversion, or the subject and body of an outgoing email) |
| `action` | Runs inside the transaction after the change; an error rolls it back |

Generic record hooks:
- `record.saving` (filter), `record.validating`, `record.deleting` and `record.archiving` (veto, the last one for custom records and organizations) and `record.saved` (action) run inside the transaction.
- `record.committed` runs after the commit; its errors are only logged.
- `record.deleted`, `record.restored` and `record.purged` (action) run inside the transaction of the deletion, restore or purge.

Read-path hooks (serialization, query filters, schema and search) are not offered: they run on every list and response, where WebAssembly would cost too much.

`GET /api/v1/wasm-runtime/host-catalog` lists every hook and operation, with its input shape, its kind and whether its module is enabled. The full list is in [host-catalog.md](host-catalog.md).

## Backend in Rust

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib"]

[dependencies]
aphel-plugin-sdk = { git = "https://github.com/Viniciusadm/aphel", tag = "v0.1.0" } # or a path to sdk/rust/aphel-plugin-sdk
```

```rust
use aphel_plugin_sdk::{ActionCall, Ctx, Event, HookCall, HookOutcome, Plugin, PluginError, Request, Response, Result, Value, WidgetRequest, export_plugin, hooks, json};

struct Equine;

impl Plugin for Equine {
    fn on_hook(&self, ctx: &Ctx, call: &HookCall) -> Result<HookOutcome> {
        if call.is(hooks::VALIDATING, "deal") && call.changed("status").is_some_and(|change| change.new == "won") {
            let deal = ctx.records().get("deal", &call.record_id)?;
            // read the horse with ctx.records().get("equine.horse", id) and veto:
            return Err(PluginError::new("equine_exam_expired", "exames vencidos"));
        }
        Ok(HookOutcome::default())
    }

    fn on_event(&self, ctx: &Ctx, event: &Event) -> Result<()> {
        // event.event_type == "deal.won": ctx.records().update("equine.horse", id, json!({ "custom_fields": { "status": "sold" } }))
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

    fn on_action(&self, ctx: &Ctx, call: &ActionCall) -> Result<Value> {
        // call.name == "request_exam": call.input::<String>("exam"), call.record_id, call.record
        Ok(json!({ "requested": true }))
    }

    fn on_widget(&self, ctx: &Ctx, request: &WidgetRequest) -> Result<Value> {
        // request.name == "horses_for_sale": request.settings, request.period_from/period_to, request.timezone
        Ok(json!({ "horses": [] }))
    }
}

export_plugin!(Equine);
```

`Ctx` is the plugin API (every call runs with the scope described in [Capabilities](#capabilities-and-scope)):

| Call | What it does |
| --- | --- |
| `ctx.records().get/list/create/update/delete(record_type, …)` | Any record type through its record handler: permissions, field permissions, validation, hooks, events and audit apply. `list` takes a `ListQuery` (filter AST, sort, search, pagination up to 100). Inside a hook only reads are allowed (see below) |
| `ctx.records().update_if(record_type, id, expected_updated_at, values)` | Conditional update: the row is locked in the update transaction and a stale `updated_at` gives 409 `version_conflict` without writing anything. Pass the `updated_at` of the record exactly as returned; custom records return it with microseconds, core records with seconds |
| `ctx.records().apply(&[Mutation::create(..), Mutation::update(..), Mutation::update_if(..)])` | 1 to 100 creates or (conditional) updates, at most 1 MiB of values, in **one transaction**: every write, transactional hook, event and audit entry commits together or nothing does. A value `{"$ref": n}` (`Mutation::reference(n)`) is replaced by the id of the earlier mutation `n`, and reference checks see the earlier uncommitted writes. A failure rolls everything back and names the mutation (`mutations[3]: …`, fields `mutations[3].custom_fields.x`). Post-commit hooks run after the commit. Deletions and deferred batches are not supported; record types without transactional create/update answer 422 `record_operation_not_supported` |
| `ctx.can(permission)`, `ctx.permissions()` | The effective permissions of the call (actor ∩ capabilities), also inside hooks. Use them to enforce business limits on every write path, including the generic record API |
| `ctx.kv(collection).get/set/delete/list` | Private key-value storage of the plugin (`plugins_data`), JSON values, collection up to 64 characters, id up to 128 |
| `ctx.settings()`, `ctx.setting::<T>(key)` | Settings with defaults applied (secrets are left out) |
| `ctx.secret(name)` | A `secret` setting, or `None` when it is not configured; an undeclared name gives 404 `plugin_secret_not_declared` |
| `ctx.http().send(HttpRequest::new(method, url).header(..).json(&body)?)`, `ctx.http().get(url)` | Outbound HTTP (see below) |
| `ctx.emit(name, Some((record_type, record_id)), payload)` | Publishes `plugin.<key>.<name>` (declared in `[[emits]]`, record type in its `record_types`). Not allowed inside hooks (409 `plugin_write_in_hook`); 404 `plugin_event_not_declared` for an undeclared name |
| `ctx.notify(Notification::new(kind, recipient_user_id, title).body(..).record(type, id).link(..).dedupe_key(..))` | Sends a declared notification kind (see Notifications) |
| `ctx.invoke::<T>(operation, input)` | Calls a module operation declared in `operations` (see [Module operations and domain hooks](#module-operations-and-domain-hooks)). Writing operations are not allowed inside hooks (409 `plugin_write_in_hook`) |
| `ctx.defer_effect(key, command)` | Requests a durable derived mutation. Returns an intention identifier, not a record result; keys are unique within an execution |
| `ctx.hook_state()` | Explicit operation snapshots, phase, availability and effect permissions; null outside hooks |
| `ctx.execution_context()` | Host-defined executor, initiator, trigger, parent and correlation identifiers |
| `ctx.now()`, `ctx.log(level, message)` / `ctx.info(..)` | Clock and server log |

**Writes inside hooks and retryable callbacks.** Use explicit durable effects rather than independent writes. A transactional hook persists its intentions and their jobs atomically with the original record; workers execute derived mutations with transactional receipts and reauthorize them at execution time. Immediate side effects remain refused in hooks, and retryable event, schedule, and automation callbacks require `defer-effect` for mutations. Catalog entries with `external_query` (`ai.*` and `enrichment.lookup`) support synchronous `invoke` with persisted result reuse and no automatic repeat of an uncertain external request. E-mail and signature delivery operations enqueue work; their receipt confirms queue acceptance. Read `ctx.hook_state()` / `ctx.hookState()` for explicit previous and proposed snapshots; ordinary record queries remain independent persisted reads. See [Durable execution and hook state](durable-execution.md) for guarantees, commands, migration instructions, causality, and catalog readiness.

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
- In `PUT /api/v1/wasm-runtime/<key>/settings`, a string sets the secret, `null` removes it and an absent key keeps it.
- `GET /api/v1/wasm-runtime` only reports `settings[].configured`, and the audit writes `[redacted]`.
- Removed on uninstall.

**Schedules.**
- Every minute, the job `wasm_runtime.tick` enqueues one `plugin.schedule` job per due slot of each enabled plugin. It looks back 5 minutes and caps at 60 slots per tick.
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

**Automations.** Each `[[actions]]` entry is an action `plugin.<key>.<action>` in the automation editor, with a form built from its `inputs`:
- An automation with a missing required input or an invalid select value is refused (422) when it is created or published.
- At run time the templates in the inputs (`{{record.custom_fields.x}}`) are rendered first, then `on-action` receives the input, the record, the automation and run ids and the actor. The JSON it returns is the step output in the run log.
- Each `[[emits]]` entry is a trigger `plugin.<key>.<name>`: the automations listening to it run when the backend calls `emit`.
- Emitted events are not part of the public event catalog, so webhooks do not receive them yet.

**Notifications.** `notify` sends a declared kind to a user:
- The recipient must be an active user, and must be able to read the record when one is attached (403 `plugin_notification_record_not_readable`).
- The title has up to 200 characters and the body up to 2000. The link must be a path inside Aphel (`/deals/<id>`).
- The person behind the call is not notified about their own action. `dedupe_key` makes repeated sends idempotent.
- Delivery follows the user's preferences for `<key>_<name>`, through the same in-app, email and push channels as the core notifications.
- The preferences screen shows the kind key unless the web bundle labels it (see Web in React).

**Dashboard widgets.** Each `[[widgets]]` entry is a widget `<key>_<name>` in the dashboard catalog:
- `on-widget` receives the saved settings (a JSON object up to 16 KB, `default_settings` when empty), the period, the time zone and the viewer, and returns any JSON. It runs with the viewer's scope and a 5 s limit.
- The web bundle renders that JSON with `app.widget('<key>_<name>', { render })`; without a renderer the card shows "unsupported".

**Reading and counting.**
- `records.aggregate(query)` takes the body of `POST /api/v1/aggregates` and runs with the call's scope.
- `records.iterate(type, query, cursor)` walks a large listing in pages of up to the maximum page size. It orders by `created_at` unless the query sets a sort, and returns the next cursor or nothing at the end. The cursor is an offset, so records deleted during a walk can shift the following page.
- `kv.increment(id, delta)` and `kv.set_if_absent(id, value)` are atomic. `next_number(sequence)` hands out an increasing number outside any transaction, so a caller that fails afterwards leaves a gap.

**Company and directory.** `organization()` returns the name, currency, locale, time zone and primary color. `list_users(query)`, `list_roles()` and `list_teams()` return the directory; `list_users` lists active users only, with name, email, role and teams.

**Files.** `upload_file(type, id, name, bytes)` attaches a file to a record through the same use case as the API, with the same limits and permissions, and returns its metadata with `download_path`. `read_file(id)` returns the metadata and the bytes, up to 10 MB. Attachments of a plugin's entities are removed with its data on purge.

**Non-JSON responses.** Inside a route, `respond_bytes(content_type, filename, bytes)` replaces the JSON body. Allowed types: CSV, plain text, calendar, PDF, JSON, ZIP, octet-stream, PNG and JPEG, up to 10 MB. A filename turns the response into a download.

**Audit reason.** `set_audit_reason(reason)` fills the `reason` column of the audit entries of the writes made afterwards in that call, up to 500 characters.

**Durable email and HTTP.** `defer_effect` also takes `{ "kind": "email", to, subject, text, html?, purpose? }` and `{ "kind": "http", method, url, headers?, body? }`:
- The key makes the effect idempotent, so a retried callback does not send twice.
- Email needs `emails.send` in `requires` and the `email` module. It goes through the installation's SMTP and is skipped when the recipient is on the opt-out list (`/api/v1/communication/opt-outs`, scopes `all` and `marketing`).
- HTTP follows the `allowed_hosts` and the SSRF policy of `http-fetch`.

**Sub-minute schedules.** A `cron` with seconds runs at each second slot: `wasm_runtime.tick` enqueues the slots of the next 60 seconds, up to 12 per schedule, so the shortest useful period is 5 seconds.

**Data subject requests.** With `[privacy] subject_data = true`, each privacy request calls `on-action` with the name `privacy.export`, `privacy.anonymize` or `privacy.delete`:
- The input is `{ request_id, action, records: [{ record_type, id }] }`, and the call runs as the plugin, before the core erases the records, with a 60 s limit.
- The JSON returned by `privacy.export` goes into the export under `external.<key>`. An `affected` number is added to the report.
- The call can be repeated, so it must be idempotent. An error makes the request finish as failed, reporting the remaining data.
- Entity fields marked `personal` or `sensitive` are already found and erased by the core; this call is for data kept elsewhere, such as the key-value store. Retention policies do not make this call.

**API client.** `api` from `@aphel/web-sdk` returns the inner value when the JSON body has `data` as its only key, and the whole body otherwise. Do not read `.data` from it blindly: `{ data, meta }` keeps the envelope and `{ data }` does not.

**Public web app.** `[web] public = "web/public"` serves that folder of the package at `/p/<key>/`:
- A path without a file extension that matches no file gets `index.html`, so client-side routing works. A missing asset gives 404.
- Refer to assets by absolute path (`/p/<key>/assets/app.js`), because deep links change the base of relative ones.
- Responses carry a restrictive `Content-Security-Policy` (scripts and connections only from the same origin) and are shown in the consent dialog as a public route.
- The page shares the origin of Aphel, so its code has the same trust as the plugin's web bundle: it runs with the session of a signed-in visitor.

### The contract (WIT)

The backend is a [WebAssembly component](https://component-model.bytecodealliance.org) of the world `aphel:plugin@1.0.0`:
- It is defined in [`back/modules/wasm_runtime/wit/plugin.wit`](../../back/modules/wasm_runtime/wit/plugin.wit) and shipped inside the SDK crate as `sdk/rust/aphel-plugin-sdk/wit/plugin.wit`. The copies are kept in sync by `make modules` and checked by `make lint`.
- It is also copied to `sdk/js/aphel-plugin-sdk/wit/plugin.wit` for the JavaScript SDK.
- It exports `on-hook`, `on-event`, `on-route`, `on-schedule`, `on-action`, `on-widget`, `on-operation` and `on-contribution`.
- It imports the `host` interface: `get-record`, `list-records`, `create-record`, `update-record`, `delete-record`, `update-record-if`, `apply-records`, `can`, `permissions`, `kv-get`, `kv-set`, `kv-delete`, `kv-list`, `kv-increment`, `kv-set-if-absent`, `next-number`, `aggregate-records`, `iterate-records`, `upload-file`, `read-file`, `respond-bytes`, `set-audit-reason`, `organization-json`, `list-users`, `list-roles`, `list-teams`, `defer-effect`, `hook-state-json`, `execution-context-json`, `http-fetch`, `notify`, `emit`, `invoke`, `settings-json`, `secret`, `now` and `log`.
- Record values travel as JSON strings inside the typed records, because record fields are dynamic (custom fields, custom objects).

`aphel-plugin-sdk` wraps the bindings generated by `wit-bindgen` in the `Plugin` trait and `Ctx` above. Any language with component tooling (`wit-bindgen` for C, Go or C#; `componentize-js`; `componentize-py`) can implement the same world directly.

**Versioning.** `1.0.0` has not been released yet. From its first release on, `1.x` only accepts additions that do not change existing types: new host functions, new interfaces and new optional exports. The shape of an existing `record`, `enum` or function signature is frozen, because a component compiled against it no longer links if a field is added, removed or reordered. Such a change needs `aphel:plugin@2.0.0`. Serving two major versions at the same time is not implemented: the host binds a single world, so introducing `2.0.0` also means adding a second set of bindings to the host.

The standard library of `wasm32-wasip2` imports WASI interfaces. The host provides them with an empty sandbox (no filesystem, no environment, no stdio, no sockets), so only clocks and randomness work; outbound HTTP goes through `http-fetch`.

**Catalog constants.** `aphel_plugin_sdk::operations::NOTES_CREATE` and `aphel_plugin_sdk::domain_hooks::DEAL_WINNING` (JavaScript: `operations.NOTES_CREATE`, `domainHooks.DEAL_WINNING`) are generated from the compiled build's catalog by `make catalog-docs` (`aphel catalog --sdk rust|js|js-types`). They carry keys only; typed inputs and outputs wait for the operation schemas.

### Shapes you will need

- **Record JSON.** `ctx.records().get/list/create/update` return records like `{ "id", "record_type", "title", "owner_id", "created_at", "updated_at", "custom_fields": { … } }`; the values of a plugin entity's declared fields live in `custom_fields.<key>` (a `select` is its option `value`, an empty reference or value is `null`). Writes take the same nesting: `{ "custom_fields": { "status": "on_loan" }, "owner_id": "<user id>" }`.
- **Title field.** Every entity has a required `name` in `custom_fields.name` (the record title). Do not declare it; set it when you create records.
- **Hook changes.** In `record.validating` and `record.saving`, `HookCall.changes` lists `{ field, old, new }`; entity fields appear as `custom_fields.<key>` (`call.custom_changed("status")` builds that name), core fields by name. `old` is `null` on create (`call.is_create()`). `call.custom_fields` carries the proposed values of the write.
- **Errors.** `PluginError::new(code, message)` answers 422 with `error.code`; `PluginError::field(code, field, message)` answers 422 and also puts the message under `details.fields`. A route handler that returns nothing, and the default `on_route`, answer 404 `route_not_found`.
- **Operations.** `on_operation(key, input)` returns any JSON; the manifest `permission` of a provided operation is optional.
- **Cargo.** The SDK re-exports `serde_json` (`Value`, `json!`, `Map`), not `serde`; add `serde` to your crate if you derive on your own types.
- **Pack.** `aphel-plugin pack` zips `web/dist/*` as `web/*`, so a hand-written bundle goes in `web/dist/index.js`, and the manifest needs `[web] entry = "web/index.js"` (`aphel-plugin new` does not write it).

### A web screen without a bundler

A bundle can be plain JavaScript. `app.nav` needs an `icon` component (any React component that takes props such as `className` and renders an `<svg>`), `to` is an absolute path, and `app.routes` paths are relative to the app root:

```js
import { createElement as h } from 'react'
import { RecordListView, defineModule } from '@aphel/web-sdk'

const Icon = (props) =>
  h('svg', { viewBox: '0 0 24 24', width: 16, height: 16, fill: 'none', stroke: 'currentColor', strokeWidth: 2, ...props },
    h('rect', { x: 3, y: 4, width: 18, height: 14, rx: 2 }))

const ItemsPage = () => h(RecordListView, { recordType: 'equipment.item' })

export default defineModule({
  key: 'equipment',                                    // the plugin key
  register(app) {
    app.routes([{ path: 'equipment', Component: ItemsPage }])
    app.nav({ to: '/equipment', label: 'Equipamentos', icon: Icon, permission: 'equipment.item.read' })
  },
})
```

`app.nav` takes `{ to, label, icon, permission?, order?, group?, key?, badge?, mobile? }` (a plain string is shown as is). `RecordListView` takes `{ recordType, emptyAction?, toolbarActions?, bulkActions?, onRowClick?, baseFilter?, archived? }` and gives list, filters, forms and bulk actions without further code. The entity's permissions are `<plugin>.<entity>.read|create|update|delete|archive|export`.

## Backend in JavaScript or TypeScript

`sdk/js/aphel-plugin-sdk` (`@aphel/plugin-sdk`) builds the same component from JavaScript or TypeScript. The bundle is made with esbuild and turned into a component with [`componentize-js`](https://github.com/bytecodealliance/ComponentizeJS), which embeds the SpiderMonkey engine.

```bash
aphel-plugin new equine --js --sdk ../aphel/sdk/js/aphel-plugin-sdk   # package.json + src/plugin.js
aphel-plugin pack aphel-equine                                       # npm run build:backend → backend.wasm, then zip
```

```js
import { PluginError, definePlugin, hooks, ok } from '@aphel/plugin-sdk'

export default definePlugin({
  onHook(ctx, call) {
    if (call.is(hooks.VALIDATING, 'deal') && call.changed('status')?.new === 'won') {
      throw new PluginError('equine_exam_expired', 'exames vencidos')
    }
    return {}
  },
  onEvent(ctx, event) {
    if (event.type === 'deal.won') ctx.deferEffect('mark-sold', { kind: 'update', record_type: 'equine.horse', id, values: { custom_fields: { status: 'sold' } } })
  },
  onRoute(ctx, request) {
    if (request.is('GET', '/horses/{id}/pedigree')) return ok({ id: request.params.id })
  },
  onAction(ctx, call) {
    return { requested: call.input.exam }
  },
  onWidget(ctx, request) {
    return { total: ctx.records.list('equine.horse', { perPage: 1 }).total }
  },
})
```

The API mirrors the Rust SDK in camelCase. Host calls are synchronous: `ctx.records.get/list/create/update/delete/updateIf/apply` (`ctx.records.ref(n)` builds a `$ref`), `ctx.can(permission)`, `ctx.permissions()`, `ctx.kv(collection)`, `ctx.settings()`, `ctx.setting(key)`, `ctx.secret(key)`, `ctx.http.send({ method, url, headers, json })`, `ctx.notify({ kind, recipientUserId, title, … })`, `ctx.emit(name, { type, id }, payload)`, `ctx.invoke(operation, input)`, `ctx.deferEffect(key, command)`, `ctx.hookState()`, `ctx.executionContext()`, `ctx.now()` and `ctx.log(level, message)`.
- Throw a `PluginError` (`new PluginError(code, message, { status, fields })`, `PluginError.field`, `notFound`, `forbidden`) to veto or answer with an error. Any other exception becomes 422 `plugin_error`.
- Host errors are thrown as `PluginError` with the host's code and status, so they can be caught or let through.
- A route handler that returns nothing answers 404 `route_not_found`.
- Types ship in `src/index.d.ts`; a `.ts` entry is compiled by esbuild.
- The scaffold's `package.json` runs `aphel-plugin-js src/plugin.js -o backend.wasm` as `build:backend`; `aphel-plugin build` runs that script when it exists, otherwise `cargo`.

Cost compared with Rust: the component is about 12 MB, because it carries the JavaScript engine. A call takes a few milliseconds more, and compiling it on install takes a few seconds and a few hundred MB of temporary memory. After that, the compiled artifact is cached (see How it works), and a loaded JavaScript plugin costs about 45 MB, like a Rust one.

## Upgrades

Uploading a package with the key of an installed plugin upgrades it, after the same consent screen. The schema is reconciled without deleting data:
- New entities, fields and options are created. Changed labels, `required`, `searchable`, targets and option labels are updated.
- Fields, entities and select options that the new version no longer declares are **deactivated**: hidden, with their data kept. A later version that declares them again reactivates them. Enabling the plugin does not reactivate them.
- Changing the **type** of an existing field is refused with 409 `plugin_field_type_changed` before anything is applied. The values live in type-specific columns and are not converted, so declare a field with a new key instead.
- Adding `unique` rebuilds the reservations under a lock on the field (409 `plugin_field_unique_conflict` when existing records share a value); removing it drops them.
- Declared grants, preset grants for existing roles and entity default grants are reapplied.

## Web in React

The bundle is an ES module that default-exports a `defineModule` (same API as the built-in modules, see [`docs/modules/web.md`](../modules/web.md)). It imports the Aphel runtime by bare specifiers that the Aphel page maps with an import map, so there is a single React and a single router:

| Specifier | Content |
| --- | --- |
| `react`, `react/jsx-runtime`, `react-dom`, `react-dom/client`, `react-router`, `@tanstack/react-query`, `react-i18next` | The instances used by Aphel |
| `@aphel/ui` | The UI components and tokens, without session, records or i18n dependencies; also usable in public web apps. See [`docs/ui`](../ui/README.md) |
| `@aphel/web-sdk` | Stable API: `defineModule`, `NAV_ORDER`, `useModuleTranslation`, `Slot`, `api`, `useSession`, `RecordListView`, `RecordDetailPage`, `RecordFormDialog`, `RecordForm`, `useRecord`, `registerRecordResource`, formatting helpers, `customObjectPath`/`customRecordPath`, `useChangeFeed` (live changes of the plugin's records from `GET /api/v1/wasm-runtime/<key>/changes/stream`, with `GET /api/v1/wasm-runtime/<key>/changes?since=<seq>` for catching up; kept for 24 hours, and `reset` means the client must reload), feedback (`ConfirmDialog`, `notifySuccess`, `notifyError`) and UI components (`Button`, `Badge`, `Card`, `EmptyState`, `ErrorState`, `Skeleton`, `Input`, `Label`, `Tooltip`, `Avatar`, `cn`) — [`web/src/sdk/index.ts`](../../web/src/sdk/index.ts) |

```js
// vite.config.js
import tailwindcss from '@tailwindcss/vite'

export default {
  plugins: [tailwindcss()],
  build: {
    lib: { entry: 'src/index.tsx', formats: ['es'], fileName: () => 'index.js', cssFileName: 'style' },
    rolldownOptions: {
      external: ['react', 'react/jsx-runtime', 'react-dom', 'react-router', '@tanstack/react-query', 'react-i18next', '@aphel/web-sdk', '@aphel/ui'],
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
    app.recordTab('equine.horse', { key: 'equine.pedigree', label: 'pedigree.title', render: PedigreeTab })
    app.recordTab('contact', { key: 'equine.horses', label: 'contactTab', render: ContactHorsesTab })
    app.slot('record.detail.header', { recordTypes: ['equine.horse'], render: ExamBadges })
  },
})
```

The base components (`Select`, `Dialog`, `Table`, `Tabs`, `Checkbox`, `Switch`, `Field` and the ones above) are also available from `@aphel/ui`, which follows the installation's [theme](../themes/README.md); using it is optional, see [`docs/ui`](../ui/README.md). Other libraries (icons, date helpers) are bundled into the plugin. Use `useModuleTranslation(key)` instead of `useTranslation` (the typed namespaces only know the built-in modules). Do not rely on Aphel's own Tailwind classes: the host only generates the ones it uses, so anything else silently does nothing. A plugin builds its own stylesheet with a Tailwind prefix, declares it as `style = "web/style.css"` and imports it at the top of its entry:

```css
/* src/style.css */
@layer theme, utilities;
@import 'tailwindcss/theme.css' layer(theme) prefix(eq);
@import 'tailwindcss/utilities.css' layer(utilities) prefix(eq) source(none);
@import '../../aphel/web/src/sdk/theme.css';

@source './**/*.{ts,tsx}';
```

Classes are then written as `eq:flex` or `eq:md:grid-cols-2`. `sdk/theme.css` maps the Aphel tokens (`eq:bg-background`, `eq:text-muted-foreground`, `eq:rounded-md`) and the `dark:` variant, so the plugin follows the theme and the organization color. Always use the prefix and never import Tailwind's preflight: the stylesheet is global and loads after the host's, so an unprefixed `.p-4` would override responsive host classes such as `md:p-6` on every screen.

For types, point `@aphel/web-sdk` and `@/*` to an Aphel checkout in the plugin `tsconfig.json`:

```json
"paths": {
  "@aphel/web-sdk": ["../aphel/web/src/sdk/index.ts"],
  "@/*": ["../aphel/web/src/*"],
  "*": ["../aphel/web/node_modules/@types/*", "../aphel/web/node_modules/*"]
}
```

Labels for the plugin's notification kinds go in the `notifications` namespace, through `namespaces` (merged into the built-in translations):

```tsx
export default defineModule({
  key: 'equine',
  namespaces: {
    notifications: {
      'pt-BR': { kinds: { equine_exam_expiring: { label: 'Exame vencendo', description: 'Um exame do cavalo vence em breve' } } },
      en: { kinds: { equine_exam_expiring: { label: 'Exam expiring', description: 'A horse exam expires soon' } } },
    },
  },
  register(app) {
    app.widget('equine_horses_for_sale', { render: (data) => <HorsesForSale data={data} /> })
  },
})
```

The web app loads `GET /api/v1/public/wasm-runtime` before it builds its routes, and imports each enabled bundle from `/api/v1/public/wasm-runtime/<key>/web/…` (public, cached by version). Installing, enabling or disabling a plugin reloads the page.

## Tools

`sdk/cli` builds the `aphel-plugin` command (`cargo build --release` in `sdk/cli`):

| Command | What it does |
| --- | --- |
| `aphel-plugin new <key> [--js] --sdk <path to the SDK>` | Creates `aphel-<key>/` with `plugin.toml`, a `/ping` route and either the crate or (`--js`) `package.json` + `src/plugin.js` |
| `aphel-plugin build [dir]` | `npm run build:backend` when `package.json` has that script, otherwise `cargo build --release --target wasm32-wasip2` → `backend.wasm`; then `npm run build` in `web/` when it has that script |
| `aphel-plugin pack [dir] [-o file]` | Builds and zips `plugin.toml`, `backend.wasm` and `web/dist/*` (as `web/*`) into `<key>-<version>.aphelplugin` |

Install from the command line with an API token that has `system.manage`:

```bash
curl -H "Authorization: Bearer $TOKEN" -F package=@equine-2.0.0.aphelplugin "https://crm.example.com/api/v1/wasm-runtime?dry_run=true"   # what it asks for
curl -H "Authorization: Bearer $TOKEN" -F package=@equine-2.0.0.aphelplugin https://crm.example.com/api/v1/wasm-runtime                # install
```

## Administration API

| Route | Permission | |
| --- | --- | --- |
| `GET /api/v1/wasm-runtime` | `system.manage` | Installed plugins, what each declares, `backend_error` and `settings[].configured` |
| `POST /api/v1/wasm-runtime` (multipart `package`) | `system.manage` | Install or upgrade; `?dry_run=true` only returns the consent payload |
| `POST /api/v1/wasm-runtime/{key}/enable` · `/disable` | `system.manage` | Disabling hides entities, fields and permissions and stops hooks, events, schedules, routes and the web bundle; nothing is deleted |
| `PUT /api/v1/wasm-runtime/{key}/settings` | `system.manage` | Validated against the declared settings; secrets set, removed (`null`) or kept (absent) |
| `POST /api/v1/wasm-runtime/{key}/grants` | `system.manage` | Reapplies declared, preset and entity default grants to the roles that exist; never creates roles |
| `DELETE /api/v1/wasm-runtime/{key}?purge=true` | `system.manage` | Removes the plugin, its key-value data, secrets and permissions; with `purge` also its entities, records and fields |
| `GET /api/v1/wasm-runtime/host-catalog` | `system.manage` | Hooks and operations this Aphel offers to plugins, with the owning module and whether it is enabled |
| `GET /api/v1/public/wasm-runtime` | public | Web bundles of the enabled plugins |
| `ANY /api/v1/p/{key}/{path}` | declared per route | Plugin routes |
| `ANY /api/v1/public/p/{key}/{path}` | public, rate limited | Public plugin routes |

Every change publishes `plugin.installed|enabled|disabled|settings_updated|grants_reapplied|uninstalled` with audit.

## How it works

The compiled module `plugins` ([`back/modules/wasm_runtime`](../../back/modules/wasm_runtime)) is the host:

- **Storage.** The tables are `plugins_installed` (with `installed_by`), `plugins_files` (package files), `plugins_data` (key-value), `plugins_compiled` and `plugins_state.version`. Declared permissions live in the kernel table `runtime_permissions` (owner `plugin:<key>`). The version is bumped on every change, and every `serve` and `worker` process rechecks it at most every 2 s. Only the plugins whose checksum changed are recompiled.
- **Runtime.** [`wasmtime`](https://wasmtime.dev) with the Component Model, Cranelift, epoch interruption and per-instance memory limits.
  - Host bindings come from `wasmtime::component::bindgen!` over the WIT file.
  - Each plugin is compiled once into a `PluginPre` (imports already resolved), so a call only instantiates and runs.
  - Compiled code is cached by the SHA-256 of `backend.wasm`: in the process (so install and load share one compile) and in `plugins_compiled`, keyed also by the engine's compatibility hash. Other processes and restarts deserialize it instead of compiling. Artifacts of backends no longer installed are pruned on install and uninstall, and freed compile memory is returned to the OS.
  - WASI comes from `wasmtime-wasi` with an empty sandbox.
- **Hooks.** The host registers one handler per record hook (`RecordSaving`, `RecordValidating`, `RecordSaved`, `RecordDeleting`). It forwards to the enabled plugins that subscribed to that hook and record type, flagging the call as in-transaction.
- **Events.** The consumer `wasm_runtime.events` accepts the event types that some enabled plugin consumes. It skips events whose actor is the plugin itself.
- **Schedules.** The jobs `wasm_runtime.tick` and `plugin.schedule` live in `schedule.rs`.
- **Automations, notifications and widgets.** `automation.rs` contributes the plugin actions and triggers to the automation registries (`AutomationActionSource`/`AutomationTriggerSource`). Notification kinds and widgets are published to the kernel's runtime contributions (`App::replace_runtime`) (`kernel/modules/builder.rs`, `App::replace_runtime`), which the notification catalog and `WidgetRegistry::builtin()` read next to the compiled ones (`notify.rs`, `widgets.rs`).
- **Routes.** Two generic routes dispatch `/api/v1/p/{key}/…` and `/api/v1/public/p/{key}/…` to the plugin, after matching the declared route and checking its permission or rate limit.
- **Schema.** Entities are created through the custom object use cases and locked with `custom_objects.module_key`. Fields are written to `field_definitions` with `module_key`, and the boot sync only releases fields of compiled modules. `schema.rs` reconciles upgrades as described in [Upgrades](#upgrades).

## Plugin events and outgoing webhooks

Enabled plugins expose their `[[emits]]` declarations in `/api/v1/events/catalog` as `plugin.<key>.<name>`. These events support exact and wildcard outgoing webhook subscriptions. The webhook `data` is exactly the JSON object passed to `ctx.emit`, including nested values and sensitive-looking keys. Plugin authors are responsible for selecting public content; associated records are not loaded or added to the payload. Disabling or uninstalling a plugin removes its declarations after runtime refresh; existing queued deliveries retain their retry behavior.

## Entity storage and generic record routes

The common runtime model is `EntityDefinition`. Native builders, plugin `EntityDeclaration`s and objects created in administration all project into this model. `EntityStorage` distinguishes native `table`, generated `managed` and EAV `dynamic` storage. Existing plugin manifests default to `dynamic`; runtime plugins cannot choose native tables.

A declaration with `storage = "managed"` gets its own deterministic table, typed columns and declared indexes. Installation serializes schema updates with a database lock. Upgrades may add nullable columns and indexes; changing existing column types, changing index definitions, adding required columns to an existing table or switching storage backends requires a data migration. Indexes must fit the storage key limit; set `max_length` when indexing text. Removing a declared field preserves its column and data. Purging the owning plugin drops its generated tables.

[`examples/managed-entity.toml`](examples/managed-entity.toml) is a manifest fragment declaring `example_records.asset`, including a bounded name column and an index. It needs the usual plugin component to become an installable package. It has not been installed as a verification instance, in keeping with the repository's no-tests rule.

`GET /api/v1/entities` lists readable entity definitions and their storage. Record lists and mutations use `/api/v1/entities/{entity}/records` and `/api/v1/entities/{entity}/records/{id}`. Entity keys and record types use `<plugin>.<entity>`; organization-created objects use `custom.<key>`. Existing native record aliases are accepted at input boundaries and normalized. A transactional migration rewrites stored identities and dynamic permission/event prefixes. Native public event names stay stable. Managed entities use the generic handler with field validation, field permissions, record scope, hooks, audit, outbox events and custom fields. Versioned updates accept `If-Match`.

Native tables without a behaviour handler must explicitly opt into generic writes with `generic_mutations(create_permission, delete_permission)` and declare their update permission. `people` uses the default handler for transaction orchestration, generic reads, deletion and archival. Its `EntityWriteBehavior` preserves contact normalization, duplicates and company-link invariants in transactional writes. Dynamic object routes remain available for dynamic storage; managed records use the generic entity routes.

Generate a managed entity schema without a database:

```sh
cargo run --manifest-path back/Cargo.toml -p aphel -- entity-schema --key example_records.asset docs/plugins/examples/managed-entity.json
```

The JSON declaration and TOML plugin example describe the same entity. The command uses the same schema generator as installation; its SQL is recorded in `docs/plugins/examples/managed-entity.sql`.

Soft references are declared with `field`, `target` and `on_delete`. A target is `{ "kind": "entity", "entity": "example_records.asset" }` or `{ "kind": "trait", "trait_key": "inventory.asset", "type_field": "asset_type" }`. Trait references store the entity key in a companion field. Deletion policies are `restrict` (default), `clear` and `cascade`; clear fields must be optional. Policies execute in the deletion transaction and cascades are bounded to 1000 records. Dependent entities must support transactional mutations. References are validated on writes and restoration, with the actor's target read scope.

Native extensions use `RecordTypeExtension::side_table(table, record_column)`. The extender owns that table; the host provisions nullable contributed columns and joins them for query filters and sorting. A scalar expression handles snapshots and standalone query consumers. Purging the parent removes its extension rows. The `leads` module now keeps lifecycle stages in its own extension tables; upgrading copies existing stage values and leaves historical physical columns unused.
