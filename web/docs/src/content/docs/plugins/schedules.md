---
title: Schedules
description: Run plugin code on a cron schedule with second precision, with slot semantics, retries and uniqueness guarantees.
sidebar:
  order: 9
---

A schedule runs your backend at times described by a cron expression. Use it for periodic work: syncing with an external service, sending reminders, cleaning up, building summaries.

```toml
[[schedules]]
name = "nightly_sync"
cron = "0 30 2 * * *"

[[schedules]]
name = "weekday_digest"
cron = "0 0 8 * * MON-FRI"
```

| Key | Rules |
| --- | --- |
| `name` | Unique lowercase key, at most 64 characters. |
| `cron` | Cron expression with **seconds**. |

A plugin can declare at most 10 schedules, and a plugin with schedules needs a backend.

## Cron format

The expression has six fields, in this order, separated by spaces: seconds, minutes, hours, day of month, month, day of week. A seventh optional field for the year is also accepted. Each field accepts `*`, single values, lists (`1,15`), ranges (`1-5`), steps (`*/10`) and, for months and weekdays, English names (`JAN`, `MON`).

| Expression | Runs |
| --- | --- |
| `0 */5 * * * *` | Every five minutes, on the minute. |
| `0 0 * * * *` | Every hour. |
| `0 30 2 * * *` | Every day at 02:30. |
| `0 0 8 * * MON-FRI` | At 08:00 on weekdays. |
| `*/30 * * * * *` | Every 30 seconds. |

All times are UTC. An invalid expression is rejected when the plugin is installed: `must be a cron expression with seconds (sec min hour day month weekday)`. A five-field expression without seconds is not valid.

## The handler

`on-schedule(name, slot)` is called with the schedule name and the **slot**: the exact scheduled time as an RFC 3339 timestamp with second precision, for example `2026-11-06T02:30:00Z`.

```js
onSchedule(ctx, name, slot) {
  if (name === 'nightly_sync') {
    ctx.deferEffect(`sync:${slot}`, {
      kind: 'http',
      method: 'POST',
      url: 'https://api.example.com/sync',
      body: '{}',
    })
  }
}
```

The handler runs as the plugin, with the permissions of the user who installed it limited to the plugin's capabilities (`503 plugin_owner_inactive` if that user is gone). It has 60 seconds. Because schedules are retried, it can only perform side effects through `defer-effect`; direct writes and `http-fetch` fail with `409 plugin_effect_required`. See [Events](/docs/plugins/events/#side-effects-in-event-handlers).

## Slot semantics

A scheduler job runs every minute. For each enabled plugin that has a backend, it computes the slots of every schedule and enqueues one job per slot.

- **Due slots:** slots from the last five minutes that have not run yet are enqueued, so a short outage of the worker does not skip them. A slot older than five minutes is dropped. At most 60 due slots per schedule are enqueued per tick.
- **Upcoming slots:** slots in the next 60 seconds are enqueued with their exact run time, which is what gives schedules with seconds their precision. At most 12 upcoming slots per schedule are enqueued per tick, so a schedule that fires more than about twelve times a minute is throttled.
- **Uniqueness:** a slot is identified by plugin, schedule name and slot time. The same slot is never enqueued twice, and the execution record of a slot is created once, so concurrent workers or repeated ticks do not run it twice.
- **Retries:** a failed slot is retried up to three times. A slot that completed is never run again.
- **Disabled or removed schedules:** a job for a schedule that no longer exists in the manifest, or for a plugin that was uninstalled, does nothing.
- **Overlap:** two different slots of the same schedule can run at the same time if one takes longer than the interval.

The slot is the right idempotency key for work done by a slot: use it in deferred effect keys, as in the example above.

## Practical guidance

- Keep the work of one slot below the 60 second limit. Split large jobs: process a page of records per slot using `iterate-records`, and store the cursor with a deferred `kv_set` effect.
- Schedules are run by workers. If Aphel is started with `aphel serve` and no worker (`aphel worker` or `aphel serve --with-worker`), no slot runs.
- A schedule runs for every enabled plugin regardless of user activity. Do not assume anyone is signed in.
