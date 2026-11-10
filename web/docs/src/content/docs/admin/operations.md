---
title: Operations
description: Health endpoints, metrics, the worker and scheduler, background jobs and retries, the System screen, audit and logs.
sidebar:
  order: 12
---

This page covers running an installation day to day: how to tell it is healthy, what the background machinery does, and where to look when something fails.

## Processes

| Process | Command | Does |
| --- | --- | --- |
| Server | `aphel serve` | HTTP API and the web app. Also applies migrations at start unless `RUN_MIGRATIONS` is false |
| Worker | `aphel worker` | Background jobs, the scheduler and the worker heartbeat |
| Combined | `aphel serve --with-worker` | Both in one process. Ignored, with a warning, when `DATABASE_URL` is not set |

Keep a worker running. Without one, queued jobs and event consumers do not run, schedules do not fire, and `/_aphel/health/ready` fails. On a shutdown signal both processes stop gracefully, and the worker waits for the jobs it is running.

## Health endpoints

| Endpoint | Auth | Answer |
| --- | --- | --- |
| `GET /_aphel/health` | none | `200` and the text `ok` when the process is up |
| `GET /_aphel/health/ready` | none | `200` when every check passes, `503` otherwise, with a JSON body and `Cache-Control: no-store` |
| `GET /_aphel/metrics` | `Authorization: Bearer <METRICS_TOKEN>` | Prometheus text. `404` when `METRICS_TOKEN` is not set, `401` for a wrong token |

The readiness body has this shape:

```json
{
  "status": "ok",
  "checks": [
    { "name": "database", "status": "ok" },
    { "name": "storage", "status": "ok" },
    { "name": "worker", "status": "ok", "age_seconds": 4, "threshold_seconds": 90 },
    { "name": "scheduler", "status": "ok", "age_seconds": 0, "threshold_seconds": 120 },
    { "name": "queue", "status": "ok", "age_seconds": 0, "threshold_seconds": 300 }
  ],
  "checked_at": "2026-11-06T12:00:00Z"
}
```

| Check | Fails when | Threshold variable |
| --- | --- | --- |
| `database` | The database cannot be reached (`unreachable`) | none |
| `storage` | A probe file cannot be written (`unavailable`) | none |
| `worker` | No heartbeat, or the latest is older than the limit (`no_heartbeat`, `heartbeat_stale`) | `HEALTH_WORKER_MAX_SILENCE_SECS`, default 90 |
| `scheduler` | The scheduler never ran, or the most overdue schedule is late (`never_started`, `schedules_overdue`) | `HEALTH_SCHEDULER_MAX_DELAY_SECS`, default 120 |
| `queue` | The oldest ready job waited longer than the limit (`backlog_delayed`) | `HEALTH_QUEUE_MAX_DELAY_SECS`, default 300 |

When the database is down, the worker, scheduler and queue checks fail with `database_unreachable`. With `HEALTH_REQUIRE_WORKER=false`, the worker and scheduler checks are `skipped`.

:::caution
By default, an installation with only `aphel serve` (no worker) is not ready: `/_aphel/health/ready` returns `503` because no worker heartbeat exists. Either run a worker or set `HEALTH_REQUIRE_WORKER=false`.
:::

The Docker image uses `aphel healthcheck` for the server and `aphel healthcheck --worker` for the worker. The first calls `/_aphel/health` on the local address. Add `--ready` to call `/_aphel/health/ready` instead. The worker check reads the heartbeat table and, when the host name is known, considers only the worker of the same host. Exit status is 0 when healthy.

## Metrics

With `METRICS_TOKEN` set, `GET /_aphel/metrics` serves Prometheus text format 0.0.4. Example scrape configuration:

```yaml
scrape_configs:
  - job_name: aphel
    metrics_path: /_aphel/metrics
    authorization:
      credentials: replace-with-the-metrics-token
    static_configs:
      - targets: ["aphel.example.com:3001"]
```

| Metric | Type | Meaning |
| --- | --- | --- |
| `aphel_build_info{version}` | gauge | Always 1, labelled with the version |
| `aphel_database_up` | gauge | 1 when the database answered |
| `aphel_jobs{queue,state}` | gauge | Jobs by queue and state: `ready`, `scheduled`, `running`, `failed` |
| `aphel_jobs_failed_by_kind{kind}` | gauge | Jobs that exhausted their attempts, by kind |
| `aphel_jobs_oldest_ready_age_seconds` | gauge | Age of the oldest ready job, 0 if none |
| `aphel_scheduler_overdue_seconds` | gauge | How late the most overdue schedule is |
| `aphel_workers_alive` | gauge | Workers with a recent heartbeat |
| `aphel_worker_heartbeat_age_seconds` | gauge | Seconds since the latest heartbeat |
| `aphel_http_responses_total{class}` | counter | Responses served by this process, by class `1xx` to `5xx` |

The HTTP counter belongs to the process that serves `/_aphel/metrics`. The job and worker gauges come from the database and are the same from any process.

## The worker

The worker polls the job queue, runs jobs with the handlers that enabled modules register, runs the scheduler, and records a heartbeat in the database every 15 seconds (`WORKER_HEARTBEAT_INTERVAL_SECS`). Heartbeats of workers that disappeared are cleaned up. The tuning variables are `WORKER_CONCURRENCY` (4), `WORKER_POLL_INTERVAL_MS` (1000) and `WORKER_JOB_TIMEOUT_SECS` (300). See [Configuration](/_aphel/docs/admin/configuration/).

Disabling a module stops its jobs and consumers: they do not run while the module is off.

## Scheduler

Modules register recurring schedules with cron expressions. At worker start the schedules are written to the database, and the scheduler checks every `SCHEDULER_INTERVAL_SECS` (5) for those that are due and enqueues a job for each. Examples of what runs this way: mailbox synchronization (a dispatcher every minute), cleanup of old jobs and idempotency keys, and the recurring work of many modules.

If a schedule is overdue by more than `HEALTH_SCHEDULER_MAX_DELAY_SECS`, the readiness check fails.

## Jobs and retries

Background work is a **job** in a database queue, for example delivering webhooks, running automations or running event consumers.

- A job has a kind, a queue (`default` unless the module says otherwise) and a maximum number of attempts (5 unless the module says otherwise).
- When a job fails and attempts remain, it is retried with exponential backoff: 10 seconds after the first failure, doubling each time, capped at 1 hour.
- When attempts are exhausted, the job is marked failed and keeps its last error. Payloads are not shown in the interface.
- A job locked longer than `WORKER_JOB_TIMEOUT_SECS` plus 60 seconds is treated as lost (for example after a crash). It goes back to the queue, or fails when it has no attempts left.
- Finished jobs are deleted after 7 days and failed ones after 30 days, by an hourly cleanup job (minute 17).

## The System screen

Open the System screen. It needs the `system.manage` permission, which administrators have. If the organization requires two-factor authentication for you, enable it first.

| Section | Shows |
| --- | --- |
| Status | The readiness checks, the Aphel version, the operating system and architecture |
| Workers | Each worker with its last heartbeat and start time. Empty means no worker is running |
| Job queue | Counts of ready, scheduled, running and failed jobs, the age of the oldest ready job, and a breakdown by kind |
| Failed jobs | Jobs that exhausted their attempts, with the last error. **Retry** resets the attempts and puts the job back in the queue. **Discard** deletes it. Both are recorded |
| Recent failures | Failures of automations, outgoing webhooks and integrations, with metadata only |
| Migrations | Applied and pending migrations and any problems in the history |
| Compiled modules | Version, compatibility, migrations and whether each is enabled |

The same information is available at `GET /api/v1/system/overview`, `/system/jobs/stats`, `/system/jobs/failed`, `/system/migrations` and `/system/failures`. Retry is `POST /api/v1/system/jobs/{id}/retry` and discard is `DELETE /api/v1/system/jobs/{id}`.

## Logs

Aphel writes logs to standard error, controlled by `RUST_LOG` (default `info`). Examples:

```bash
RUST_LOG=info,sqlx=warn aphel serve
RUST_LOG=debug aphel worker
```

Every request gets an `X-Request-Id` header (a UUID, or the one the client sent), which appears in the request log lines and is returned in the response. Use it to correlate a user report with the logs. The `Authorization` and `Cookie` headers and `Set-Cookie` responses are marked sensitive and are not logged. With Docker, read them with `docker compose -f docker-compose.prod.yml logs -f app worker`; with systemd, `journalctl -u aphel -u aphel-worker`.

## Audit

The audit log records who changed what, and security events record sign-ins and permission changes. Both are described in [Security](/_aphel/docs/admin/security/). The System screen's actions on failed jobs are recorded in a separate system audit log.

## Diagnostics

`aphel doctor` prints a sanitized report you can attach to an issue: version, operating system, MySQL version, the Aphel environment variables with secrets redacted, and checks for the database, migrations, workers, scheduler, queue, storage, disk space, SMTP reachability and encryption key. See [CLI](/_aphel/docs/admin/cli/).

## Routine checklist

- Watch `/_aphel/health/ready` or the `aphel_*` metrics, and alert on failed jobs.
- Check the System screen after upgrades.
- Verify the backups are being produced and restore one now and then. See [Backup and restore](/_aphel/docs/admin/backup-and-restore/).
- Keep free disk space above `UPGRADE_MIN_FREE_DISK_MB`.
