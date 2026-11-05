# C3 and C7: what stops a native module from being replaced

Both composition tests ask for a plugin to take over what a native module provides while consumers stay unchanged. Checked on 2026-11-05 with a WebAssembly plugin that provides `tasks@1` (C3) and by reading the operation and contract catalog (C7).

## What works

- A plugin that declares `provides.contracts = ["x@1"]` holds the native module that declares the same contract: the module is disabled on install and enabled again on uninstall (only if it was enabled before).
- Consumers that call operations through `invoke` or `ctx.call` resolve native and plugin providers alike (P6 gate).

## What stops it

1. **Consumers depend on the module, not on the contract.** Installing a plugin that provides `tasks@1` is refused with 409 `plugin_module_conflict_blocked` because `sequences` declares a dependency on the `tasks` module. The held-module check treats any enabled dependent as blocking.
2. **Contracts are mostly native traits.** `tasks@1` is `tasks.complete` plus three extension points (`calendar_api`, `reminder_source`, `remote_calendars`) whose implementations are Rust traits contributed with `app.contribute`. A WebAssembly plugin cannot implement them, so it cannot be a second provider even if it holds the module.
3. **Operation keys stay owned by the module that holds them.** A plugin that provides a key such as `deals.move_stage` is refused with "already provided by a built-in module", even when its contract would hold that module.
4. **Plugin-provided names are prefixed.** Widgets are `<plugin>_<name>`, routes are `/api/v1/p/<plugin>/…`, permissions and entities are `<plugin>.<…>`. Dashboard layouts, web calls and roles that reference the native names do not follow.

## Changes that would be needed

- Declare consumers' dependencies on contracts (`uses`) and let the held-module check ignore dependents whose required contracts the plugin provides.
- Split each swappable contract into a data-shaped part (operations, events, entities) that a plugin can implement and a native-only part that stays with the host.
- Let a plugin that holds a module take over that module's operation keys while it is enabled.
- Give web resources a way to alias a native name to the plugin's (widget keys, routes).

These are contract-model decisions, not fixes; C3 and C7 stay open until they are taken.
