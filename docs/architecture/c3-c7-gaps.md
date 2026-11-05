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

`booking` and `telephony` use the native-only part of `tasks` (a call is a task row written in the call's own transaction; meetings use `MeetingScheduler` and calendar sources), so they declare a hard dependency on the `tasks` module. A plugin that provides `tasks@1` is therefore refused (409 `plugin_module_conflict_blocked`, naming the module) while either is enabled, and accepted once they are disabled. `telephony` previously declared the dependency as optional, which let a replacement through and broke call logging silently. The native-only extension points of `tasks@1` (`calendar_api`, `reminder_source`, `remote_calendars`) have no plugin implementation.

## C7: done for a module without web screens

Checked on 2026-11-05 with `reminders`, rewritten as the WebAssembly plugin `aphel-reminders-alt`, and `aphel-reminders-consumer`, a second plugin that calls `reminders.create` and `reminders.list` with `invoke`.

- New contract `reminders@1` (crate `contracts/reminders`) lists the three operations the native module already exposes; the module declares it.
- The same consumer package was used twice. With the native module it created and listed reminders; with `aphel-reminders-alt` installed (module held) the same calls were served by the plugin, stored in `reminders_alt.reminder`, and the schedule delivered the notification (`reminders_alt_due`) and marked the reminder `sent`. Uninstalling the plugin (after disabling its dependent consumer) returned the calls to the native module.
- Platform fixes found on the way, none specific to reminders:
  - Native contract providers were invisible to the plugin resolver (`uses.required` of a native contract always failed); the resolver now reads the contract descriptors of enabled modules.
  - A plugin that lists `reminders.*` in `operations` counted as a dependent of the module it replaces; operations the installing plugin provides no longer count.
  - A provided operation ran with the consumer's restricted scope, which lacks the provider's own entity permissions. It now runs with the original actor's scope (already limited by an API token, if any) intersected with the provider's capabilities, as documented.
  - The `notify` effect required `data` although the guide lists it as optional.

Follow-up, same day: the two gaps that were left for the reminders case are closed.

- **HTTP paths.** `[aliases]` in `plugin.toml` maps a native path prefix to a plugin route prefix; the module gate hands requests of a held module to the plugin's route table (`RouteTakeover` in the kernel, implemented by `wasm_runtime`). With `aphel-reminders-alt` 0.2.0, `POST/GET/PATCH/DELETE /api/v1/reminders` were served by the plugin with the native envelope, and the native paths worked again after uninstalling it.
- **Task-bound deliveries.** The plugin reads open tasks through the generic `tasks.task` entity and sends a reminder before each due time and a morning notice (08:00 in the organization time zone) as its own notification kinds (`reminders_alt_task_reminder`, `reminders_alt_task_due_today`). The reminder before due was delivered; the morning notice has the same code path but was not run because it fires at one hour of the day. The plugin's kinds differ from the native `task_due` kind, so users' notification preferences for `task_due` do not apply to them.

What is still not transparent: widget keys and web screens of a replaced module keep the plugin prefix (no alias mechanism for them yet), which does not matter for `reminders` because it has no web screens or widgets.
