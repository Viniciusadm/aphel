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

## C7: done for a module without web screens

Checked on 2026-11-05 with `reminders`, rewritten as the WebAssembly plugin `aphel-reminders-alt`, and `aphel-reminders-consumer`, a second plugin that calls `reminders.create` and `reminders.list` with `invoke`.

- New contract `reminders@1` (crate `contracts/reminders`) lists the three operations the native module already exposes; the module declares it.
- The same consumer package was used twice. With the native module it created and listed reminders; with `aphel-reminders-alt` installed (module held) the same calls were served by the plugin, stored in `reminders_alt.reminder`, and the schedule delivered the notification (`reminders_alt_due`) and marked the reminder `sent`. Uninstalling the plugin (after disabling its dependent consumer) returned the calls to the native module.
- Platform fixes found on the way, none specific to reminders:
  - Native contract providers were invisible to the plugin resolver (`uses.required` of a native contract always failed); the resolver now reads the contract descriptors of enabled modules.
  - A plugin that lists `reminders.*` in `operations` counted as a dependent of the module it replaces; operations the installing plugin provides no longer count.
  - A provided operation ran with the consumer's restricted scope, which lacks the provider's own entity permissions. It now runs with the original actor's scope (already limited by an API token, if any) intersected with the provider's capabilities, as documented.
  - The `notify` effect required `data` although the guide lists it as optional.

What this does not cover: the HTTP routes of the native module (`/api/v1/reminders`) disappear while the plugin holds it, because plugin routes live under `/api/v1/p/<key>/`; the two task-bound deliveries of the native module (reminder before a task is due, tasks due today) are not reimplemented; widget keys and web screens of other modules would still carry the plugin prefix.
