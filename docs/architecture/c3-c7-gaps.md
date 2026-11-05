# C3 and C7: replacing a native module with a plugin

Both composition tests ask for a plugin to take over what a native module provides while consumers stay unchanged. Checked on 2026-11-05 with a WebAssembly plugin that provides `tasks@1` (C3) and by reading the operation and contract catalog (C7).

## What works

- A plugin that declares `provides.contracts = ["x@1"]` holds the native module that declares the same contract: the module is disabled on install and enabled again on uninstall (only if it was enabled before).
- Consumers that call operations through `invoke` or `ctx.call` resolve native and plugin providers alike (P6 gate).

## C3: done for the task-creation consumer

Checked on 2026-11-05 with `aphel-tasks-alt`, a WebAssembly plugin that provides `tasks@1` by storing tasks in its own entity:

- The `tasks@1` contract now lists the portable operations `tasks.create`, `tasks.types` and `tasks.complete`. The native `tasks` module exposes the first two next to `tasks.complete`.
- `aphel_sdk::tasks::writer()` and `types()` use the native services while `tasks` is enabled and fall back to those operations otherwise, so consumers keep calling the same SDK functions.
- A plugin whose `provides.contracts` holds a module may provide that module's operation keys (manifest validation and runtime registration ignore the keys of held modules).
- `sequences` declares `tasks` as optional instead of a hard module dependency, so the held-module check no longer blocks it.
- Result: with the plugin installed, a sequence `task` step created its task in `tasks_alt.task` and the native table did not grow; uninstalling the plugin returned the step to the native `tasks` module. The `sequences` runner code is unchanged.

Not covered: `booking` still depends on the `tasks` module and uses native scheduling traits (`MeetingScheduler`, calendar sources), and `telephony` was not exercised. The native-only extension points of `tasks@1` (`calendar_api`, `reminder_source`, `remote_calendars`) have no plugin implementation.

## C7 and what is still open

1. **Consumers of other contracts still depend on the module.** `booking` declares a hard dependency on `tasks`; other modules do the same for their providers.
2. **Contracts are mostly native traits.** Only the data-shaped part (operations, events, entities) can be implemented by a plugin; each swappable contract needs that part defined, as was done for `tasks@1`.
3. **Plugin-provided names are prefixed.** Widgets are `<plugin>_<name>`, routes are `/api/v1/p/<plugin>/…`, permissions and entities are `<plugin>.<…>`. Dashboard layouts, web calls and roles that reference the native names do not follow, so rewriting a module that has its own web screens or widgets is not yet transparent to consumers.

C7 stays open until a module is rewritten end to end.
