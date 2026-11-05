# P5.23 operation bridges

- The 75 operations that 25 modules exposed to WebAssembly plugins through `plugin_bridge.rs` now register with `app.operation(key, OperationSpec, handler)`. One declaration feeds `ctx.call`, plugin `invoke`, the platform catalog and the legacy `PluginOperation` projection. Keys, descriptions, read/write and durability are unchanged; `ai` queries use `OperationSpec::external`.
- Inputs and outputs are still `serde_json::Value`: the keys are `OperationKey<Value, Value>` declared next to the handlers (`exposed_operations.rs`, or `plugin_bridge.rs` for the seven modules that also forward hooks). Typed inputs, outputs and JSON schemas per module remain to be written as the contracts are split (P6.6).
- Seven files stay named `plugin_bridge.rs` (`automations`, `deals`, `email`, `leads`, `quotes`, `sequences`, `tasks`): they hold the native-hook forwarders (`forward_plugin_hook`) and `PluginHook` metadata. Moving those to bus-level subscriptions is part of the WebAssembly runtime parity work (P6.1, P6.3).
- No section 5.4 bridge plugin was needed.
- [ ] Manual acceptance: from a WebAssembly plugin call one read and one write operation of each converted module (`invoke`) and confirm the result and the durable-mutation behaviour match the previous release; confirm `aphel catalog` lists each operation once.

## Kernel paths through the SDK

- Plugins no longer name `aphel_kernel::<area>::<module>` paths: `aphel_sdk::platform` re-exports the 80 kernel modules and items plugins use (`platform::application::shared`, `platform::domain::access`, `platform::database::transaction`, ...), and 3,198 references in 767 plugin files were rewritten to it. The facade is a list of module-level re-exports, not a curated item list: narrowing each entry to the items plugins need, and replacing entries with purpose-built SDK modules, is ongoing curation work. Kernel moves now need one edit in `sdk/src/platform.rs`.

## Runtime handle instead of AppState

- Plugins no longer call `ctx.state()`, `runtime_state()` or `env.state()`. `aphel_sdk::Runtime` (from `ctx.runtime()`, `env.runtime()` or `aphel_sdk::runtime()`) exposes only what plugins use: `ctx(scope)`, `db`, `registry`, `record_types`, `app`, `events`, `audit`, `jobs`, `storage`, `secrets`, `configuration`, `public_base_url`, `outbound_http_network`, `max_upload_bytes`, `mailer`, `secure_cookies` and `auth_use_cases`. Kernel helpers that used to take `&AppState` (`field_policies`, `redacted`, `field_redaction`, `file_use_cases`, `aggregate_ports`) accept `&impl AsRef<AppState>`, and `RecordCtx::from_runtime` builds a record context from it. The automation gateway still holds an `AppState` internally (its contract trait signatures take one).
- Worker handlers, hooks and widgets now hold a `Runtime` and call `runtime.ctx(&scope)`.

## Foreign record types

- The metric `plugins_with_literal_record_types` now counts only dotted record keys (`people.person`, `deals.deal`, ...) of another plugin, outside comments and OpenAPI examples. The old short keys (`contact`, `deal`, ...) were removed in P3.4 and matched JSON field names. Plugins reference peers through `aphel_contract_crm::keys::*` constants (transitional crm contract) instead of string literals; splitting that contract per capability is still open.
