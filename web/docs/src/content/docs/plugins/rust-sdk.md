---
title: Rust SDK
description: Write a plugin backend in Rust with aphel-plugin-sdk, from scaffold and build target to the Plugin trait and the Ctx API.
sidebar:
  order: 14
---

`aphel-plugin-sdk` is a Rust crate for writing plugin backends. You implement the `Plugin` trait, export it with `export_plugin!` and build a WebAssembly component for the target `wasm32-wasip2`. Rust backends are small and fast to start, which makes them a good fit for hooks, which have a two second budget.

The crate lives in the Aphel repository at `sdk/rust/aphel-plugin-sdk`. It is not published to crates.io.

## Setup

1. Install the target and build the command line tool.

```bash
rustup target add wasm32-wasip2
cargo build --release --manifest-path /path/to/aphel/sdk/cli/Cargo.toml
export PATH="/path/to/aphel/sdk/cli/target/release:$PATH"
```

2. Scaffold a project.

```bash
aphel-plugin new library --sdk /path/to/aphel/sdk/rust/aphel-plugin-sdk
cd aphel-library
```

The command creates:

| File | Content |
| --- | --- |
| `plugin.toml` | The same minimal manifest as for JavaScript. |
| `Cargo.toml` | Crate `aphel-plugin-library`, edition 2024, `crate-type = ["cdylib"]`, a path dependency on the SDK, a release profile with `opt-level = "s"` and `lto = true`, and an empty `[workspace]` table so the project builds on its own. |
| `src/lib.rs` | A plugin whose `on_route` answers `{ "pong": "<path>" }`. |

## Build

`aphel-plugin build` runs `cargo build --release --target wasm32-wasip2` and copies `target/wasm32-wasip2/release/<crate name with underscores>.wasm` to `backend.wasm`. `aphel-plugin pack` does the same and then writes `<key>-<version>.aphelplugin`. The build fails with `could not copy ...` when the crate name does not match the produced file; keep `[lib]` as in the scaffold.

If the directory has a `package.json` with a `build:backend` script, the tool runs that instead of Cargo. Keep Rust and JavaScript projects in separate directories.

## The `Plugin` trait

Implement the callbacks you need. Every method has a default: callbacks that return values answer a not-found plugin error, the others do nothing.

```rust
use aphel_plugin_sdk::{
    Ctx, HookCall, HookOutcome, ListQuery, Plugin, PluginError, Request, Response, Result,
    export_plugin, hooks, json,
};

const BOOK: &str = "library.book";

struct Library;

fn compact_isbn(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-')
        .collect()
}

fn is_valid_isbn(value: &str) -> bool {
    value.is_empty()
        || (matches!(value.len(), 10 | 13) && value.chars().all(|character| character.is_ascii_digit()))
}

impl Plugin for Library {
    fn on_hook(&self, _ctx: &Ctx, call: &HookCall) -> Result<HookOutcome> {
        let isbn = call.values.get("isbn").and_then(|value| value.as_str());
        if let Some(isbn) = isbn {
            if call.is(hooks::SAVING, BOOK) {
                return Ok(HookOutcome::default().set("isbn", json!(compact_isbn(isbn))));
            }
            if call.is(hooks::VALIDATING, BOOK) && !is_valid_isbn(isbn) {
                return Err(PluginError::field(
                    "invalid_isbn",
                    "isbn",
                    "An ISBN has 10 or 13 digits.",
                ));
            }
        }
        Ok(HookOutcome::default())
    }

    fn on_route(&self, ctx: &Ctx, request: &Request) -> Result<Response> {
        if request.is("GET", "/summary") {
            let page = ctx.records().list(BOOK, ListQuery::default().per_page(1))?;
            return Ok(Response::ok(json!({ "total": page.total })));
        }
        Err(PluginError::not_found("route_not_found", "No such route."))
    }
}

export_plugin!(Library);
```

| Method | Arguments | Returns |
| --- | --- | --- |
| `on_hook` | `&HookCall` | `Result<HookOutcome>`. Build the outcome with `HookOutcome::default().set(field, value).set_custom(field, value)`. |
| `on_event` | `&Event` | `Result<()>` |
| `on_route` | `&Request` | `Result<Response>`. The default answers `404 route_not_found`. |
| `on_schedule` | `name: &str`, `slot: &str` | `Result<()>` |
| `on_action` | `&ActionCall` | `Result<Value>`. Default: `404 action_not_found`. |
| `on_widget` | `&WidgetRequest` | `Result<Value>`. Default: `404 widget_not_found`. |
| `on_operation` | `key: &str`, `input: Value` | `Result<Value>`. Default: `404 operation_not_found`. |
| `on_contribution` | `point`, `name`, `input: Value` | `Result<Value>`. Default: `404 contribution_not_found`. |

## Types

| Type | Fields and helpers |
| --- | --- |
| `HookCall` | `hook`, `record_type`, `record_id`, `mode: Option<String>`, `actor`, `changes: Vec<Change>`, `values: Map`, `custom_fields: Map`; `is(hook, type)`, `changed(field)`, `custom_changed(field)`, `is_create()` |
| `Change` | `field`, `old`, `new` as `serde_json::Value` |
| `Actor` | `kind: ActorKind` (`User`, `ApiToken`, `Automation`, `Plugin`, `System`, `Anonymous`), `id`, `user_id` |
| `Event` | `id`, `event_type`, `record_type`, `record_id`, `actor`, `occurred_at`, `payload` |
| `Request` | `method`, `path`, `route`, `params: Map`, `query: Map`, `headers: Vec<(String, String)>`, `body: Value`, `actor`; `is(method, route)`, `param(name)`, `query(name)`, `header(name)`, `header_values(name)`, `body::<T>()` |
| `Response` | `status`, `body`; `Response::ok(value)`, `created(value)`, `with_status(status, value)` |
| `ActionCall` | `name`, `input: Map`, `record_type`, `record_id`, `record: Value`, `automation_id`, `run_id`, `actor`; `input::<T>(key)` |
| `WidgetRequest` | `name`, `settings`, `period_from`, `period_to`, `timezone`, `actor` |
| `ListQuery` | `filter`, `sort`, `search`, `page`, `per_page`; `ListQuery::eq(field, value)`, `.filter(value)`, `.per_page(n)` |
| `Page` | `data: Vec<Value>`, `total` |
| `Mutation` | `Mutation::create(type, values)`, `update(type, id, values)`, `update_if(type, id, expected, values)`, `Mutation::reference(index)` |
| `Notification` | `Notification::new(kind, recipient, title)`, `.body()`, `.record()`, `.link()`, `.dedupe_key()` |
| `HttpRequest`, `HttpResponse` | `HttpRequest::new(method, url)`, `.header()`, `.bearer(token)`, `.json(&value)?`, `.body()`; the response has `status`, `headers`, `body`, `is_success()`, `header(name)`, `text()`, `json::<T>()?` |
| `EffectCommand` | Enum for `ctx.defer_effect`: `Create`, `Update`, `Delete`, `Archive`, `Restore`, `KvSet`, `KvDelete`, `Emit`, `Notify`, `Invoke` |
| `HookState`, `ExecutionContext` | The shapes returned by `ctx.hook_state()` and `ctx.execution_context()` |

Constants: `hooks::{SAVING, VALIDATING, SAVED, DELETING, ARCHIVING, COMMITTED, DELETED, RESTORED, PURGED}`, `domain_hooks::*` and `operations::*`. The crate re-exports `serde_json`, `Value`, `Map` and the `json!` macro.

## The `Ctx`

`Ctx` gives access to the host. Each method returns `Result<T>` unless noted.

| Call | Host function |
| --- | --- |
| `ctx.records().get(type, id)` -> `Option<Value>` | `get-record` |
| `ctx.records().list(type, ListQuery)` -> `Page` | `list-records` |
| `ctx.records().create(type, values)`, `.update(type, id, values)`, `.delete(type, id)` | record writes |
| `ctx.records().update_if(type, id, expected_updated_at, values)` | `update-record-if` |
| `ctx.records().apply(&[Mutation])` -> `Vec<Value>` | `apply-records` |
| `ctx.records().aggregate(query)` | `aggregate-records` |
| `ctx.records().iterate(type, ListQuery, cursor: Option<&str>)` -> `(Vec<Value>, Option<String>)` | `iterate-records` |
| `ctx.kv(collection)` with `get::<T>`, `set`, `delete`, `list`, `increment`, `set_if_absent` | key-value storage |
| `ctx.next_number(sequence)` -> `u64` | `next-number` |
| `ctx.http().send(HttpRequest)`, `.get(url)` | `http-fetch` |
| `ctx.notify(Notification)` | `notify` |
| `ctx.emit(name, Option<(type, id)>, payload)` | `emit` |
| `ctx.invoke::<T>(operation, input)` | `invoke` |
| `ctx.defer_effect(key, command)` -> effect id | `defer-effect` |
| `ctx.settings()`, `ctx.setting::<T>(key)` | `settings-json` |
| `ctx.secret(key)` -> `Option<String>` | `secret` |
| `ctx.can(permission)` -> `bool`, `ctx.permissions()` -> `Vec<String>` | not fallible |
| `ctx.upload_file(...)`, `ctx.read_file(id)` -> `(Value, Vec<u8>)`, `ctx.respond_bytes(content_type, filename, body)` | file functions |
| `ctx.set_audit_reason(Option<&str>)` | `set-audit-reason` |
| `ctx.organization()`, `ctx.list_users(query)`, `ctx.list_roles()`, `ctx.list_teams()` | directory |
| `ctx.hook_state()`, `ctx.execution_context()` | context |
| `ctx.now()`, `ctx.log(level, message)`, `ctx.info(message)` | `now`, `log` |

Which calls are allowed where is in [Host API](/docs/plugins/host-api/#where-side-effects-are-allowed).

## Errors

`PluginError` has `code`, `message`, `status: Option<u16>` and `fields: Vec<FieldError>`. Constructors: `PluginError::new(code, message)`, `field(code, field, message)`, `not_found`, `forbidden`, `with_status(status)` and `internal(message)` (code `plugin_error`). Host errors come back as `Err(PluginError)` with the host's code and status; return them with `?` to pass them on. The mapping of codes and statuses to the caller is in [Hooks](/docs/plugins/hooks/#rejecting-with-errors).

A panic, an out-of-bounds access or running out of memory traps the instance. The caller receives `503 plugin_failed`; the details are written to the server log. Return errors instead of panicking.

## Size and performance

- A release build with `opt-level = "s"` and LTO is typically small. Avoid heavy dependencies; they are linked into every call's instance.
- Each call starts a new instance, so global state does not persist. Use records or key-value storage.
- The SDK is synchronous, like the host API. Do not use threads or async runtimes.
- The target is `wasm32-wasip2`. Crates that need operating system threads, sockets or a file system will not work, because the sandbox is empty.
