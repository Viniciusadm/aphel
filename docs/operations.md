# Operations

A guide for those who keep an Aphel installation running: processes, services, logs, scaling, health, metrics, diagnostics and upgrades. The installation itself is covered in [Installation](installation.md) and the variables in [Configuration](configuration.md).

## Processes

Aphel is a single binary, with subcommands:

| Command | What it does |
| --- | --- |
| `aphel serve` | HTTP server (API, embedded frontend, `/health`, `/health/ready`, `/metrics`) |
| `aphel serve --with-worker` | HTTP server and worker in the same process (small installations) |
| `aphel worker` | Job queue worker and scheduler for recurring jobs |
| `aphel migrate` | Applies pending migrations and exits |
| `aphel upgrade` | Guided upgrade: requirements, pending migrations, backup confirmation ([Upgrading](upgrading.md#guided-upgrade-with-aphel-upgrade)) |
| `aphel doctor` | Diagnostic report without secrets, to attach to issues |
| `aphel healthcheck` | Local probe for container healthchecks (exits 0 or 1, see [Healthcheck without curl](#healthcheck-without-curl-aphel-healthcheck)) |

**Without a running worker, nothing happens in the background**: event dispatch, notifications and emails, reminders, automations, outgoing webhooks, imports, exports, search index and cleanups. The worker writes a liveness signal (heartbeat) every `WORKER_HEARTBEAT_INTERVAL_SECS` seconds to the `system_heartbeats` table, and `/health/ready` complains when it disappears.

## systemd services

The units from [Manual installation](installation.md#5-systemd-services) separate HTTP and worker:

```ini
# /etc/systemd/system/aphel.service
[Unit]
Description=Aphel CRM (HTTP)
After=network-online.target mysql.service
Wants=network-online.target

[Service]
User=aphel
Group=aphel
WorkingDirectory=/var/lib/aphel
EnvironmentFile=/etc/aphel/aphel.env
ExecStart=/usr/local/bin/aphel serve
Restart=on-failure
KillSignal=SIGTERM
TimeoutStopSec=60

[Install]
WantedBy=multi-user.target
```

```ini
# /etc/systemd/system/aphel-worker.service
[Unit]
Description=Aphel CRM (worker)
After=network-online.target mysql.service
Wants=network-online.target

[Service]
User=aphel
Group=aphel
WorkingDirectory=/var/lib/aphel
EnvironmentFile=/etc/aphel/aphel.env
ExecStart=/usr/local/bin/aphel worker
Restart=on-failure
KillSignal=SIGTERM
TimeoutStopSec=600

[Install]
WantedBy=multi-user.target
```

The worker finishes in-progress jobs before exiting (SIGTERM) and deletes its own heartbeat. Increase `TimeoutStopSec` if there are long jobs (large imports).

### Periodic tasks (cron or timer)

Aphel does not need cron to work: cleanups, reminders, digests, retention and reindexing are recurring jobs of its own scheduler (inside the worker). Use cron or a systemd timer only for what lives outside Aphel:

```cron
# daily backup at 3 a.m. (see docs/backup.md)
0 3 * * * aphel sh -c 'set -a; . /etc/aphel/aphel.env; exec /opt/aphel/backup.sh --output /srv/backups/aphel'
# health report every 5 min for your monitoring
*/5 * * * * curl -fsS http://127.0.0.1:3001/health/ready > /dev/null || logger -t aphel "aphel not ready"
```

## Logs

- Logs go to **stderr** as text (`tracing`), one line per event, with a `request_id` on each HTTP request (the same value comes back in the `X-Request-Id` header).
- The filter is the `RUST_LOG` variable (default `info`). Examples: `RUST_LOG=warn`, `RUST_LOG=info,aphel=debug`, `RUST_LOG=info,sqlx=warn`.
- With systemd: `journalctl -u aphel -u aphel-worker -f`. With Docker: `docker compose -f docker-compose.prod.yml logs -f app worker`.
- The `Authorization`, `Cookie` and `Set-Cookie` headers are marked as sensitive and do not appear in request logs. Secrets (passwords, tokens, keys) are never logged.
- There is no native JSON format for now; for aggregators (Loki, Elastic), collect stderr as text.

## Scaling

- **Multiple workers**: run as many `aphel worker` processes as you want, on one or several machines. The queue uses `FOR UPDATE SKIP LOCKED`, so each job is reserved by a single worker. `WORKER_CONCURRENCY` controls the parallel jobs per process.
- **Scheduler**: each worker also runs the recurring job scheduler. With multiple workers, they compete for the same row with `SKIP LOCKED` and each firing has a unique key, so a recurring job is never enqueued twice. There is no need to elect a scheduler or configure anything else.
- **Multiple HTTP servers**: `aphel serve` keeps no session state in memory (sessions live in the database), so it can sit behind a load balancer. Exceptions: the rate limits (login, API tokens, public forms) and the `/metrics` response counter are per process.
- **Database**: one MySQL 8 for everything. The per-process pool has up to 30 connections; add up the processes to size `max_connections`.
- **Files**: with more than one machine, use S3 in Settings → File storage (the local disk is not shared).

## Health

| Route | Use | Response |
| --- | --- | --- |
| `GET /health` | Liveness: the process responds | `200 ok` (text) |
| `GET /health/ready` | Readiness: the installation is able to work | `200` when everything is ok, `503` if any check failed |

`/health/ready` is public and carries no business data, only the state of each check:

```json
{
  "status": "fail",
  "checks": [
    { "name": "database", "status": "ok", "reason": null, "age_seconds": null, "threshold_seconds": null },
    { "name": "storage", "status": "ok", "reason": null, "age_seconds": null, "threshold_seconds": null },
    { "name": "worker", "status": "fail", "reason": "no_heartbeat", "age_seconds": null, "threshold_seconds": 90 },
    { "name": "scheduler", "status": "fail", "reason": "never_started", "age_seconds": null, "threshold_seconds": 120 },
    { "name": "queue", "status": "ok", "reason": null, "age_seconds": 3, "threshold_seconds": 300 }
  ],
  "checked_at": "2026-10-21T12:00:00.000Z"
}
```

| Check | Fails when | `reason` |
| --- | --- | --- |
| `database` | `SELECT 1` does not respond | `unreachable` |
| `storage` | it cannot write and delete a test file (`system/health/…`) | `unavailable` |
| `worker` | there is no heartbeat or the most recent one is older than `HEALTH_WORKER_MAX_SILENCE_SECS` | `no_heartbeat`, `heartbeat_stale` |
| `scheduler` | the most overdue recurring schedule has exceeded `HEALTH_SCHEDULER_MAX_DELAY_SECS` or has never run | `schedules_overdue`, `never_started` |
| `queue` | the oldest ready job has been waiting for more than `HEALTH_QUEUE_MAX_DELAY_SECS` | `backlog_delayed` |

| Variable | Default | Effect |
| --- | --- | --- |
| `WORKER_HEARTBEAT_INTERVAL_SECS` | `15` | Worker heartbeat interval |
| `HEALTH_WORKER_MAX_SILENCE_SECS` | `90` | Maximum worker silence before failing |
| `HEALTH_SCHEDULER_MAX_DELAY_SECS` | `120` | Maximum delay of recurring schedules |
| `HEALTH_QUEUE_MAX_DELAY_SECS` | `300` | Maximum wait of the oldest ready job |
| `HEALTH_REQUIRE_WORKER` | `true` | `false` skips (`skipped`) the worker and scheduler checks, for when HTTP readiness should not depend on the worker |

In load balancers and orchestrators, use `/health` for liveness and `/health/ready` for readiness. Keep in mind that readiness depends on the worker: if the worker goes down, all HTTP replicas become "not ready" (turn this off with `HEALTH_REQUIRE_WORKER=false` if you prefer to monitor the worker separately).

### Healthcheck without curl: `aphel healthcheck`

The Docker image does not have `curl`. The binary itself performs the probe and exits with code 0 (healthy) or 1 (unhealthy), with a 3-second timeout:

| Command | What it checks |
| --- | --- |
| `aphel healthcheck` | `GET http://127.0.0.1:$HTTP_PORT/health` (liveness). If `HTTP_HOST` is a specific IP, it uses that IP |
| `aphel healthcheck --ready` | `GET /health/ready` (readiness: database, storage, worker and scheduler) |
| `aphel healthcheck --worker` | Reads from the database the latest heartbeat of the worker **on this machine** (`hostname`, which in Docker is the container id) and fails if it is older than `HEALTH_WORKER_MAX_SILENCE_SECS` |

`docker-compose.prod.yml` uses `aphel healthcheck` on `app` (liveness: it does not take HTTP down when only the worker is stopped) and `aphel healthcheck --worker` on `worker`; `worker` only starts after `app` is healthy (`app` applies the migrations on startup). In Kubernetes or another orchestrator, use `aphel healthcheck` as the liveness probe and `aphel healthcheck --ready` as the readiness probe (or the HTTP routes directly). `--worker` needs `DATABASE_URL` and opens a short connection on each run; keep the interval in the tens of seconds.

## System panel

The **System** page (`/system`) and the `/api/v1/system/*` routes require the `system.manage` permission, granted to the admin role (everyone else gets 403 `permission_denied`). They show, with no business data:

- version, OS/architecture, readiness checks and active workers (`/system/overview`);
- queue by state and by job kind, with the age of the oldest ready job (`/system/jobs/stats`);
- failed jobs with **retry** (`POST /system/jobs/{id}/retry`, resets the attempts) and **discard** (`DELETE /system/jobs/{id}`). Metadata only: kind, queue, attempts and last error, never the payload;
- recent failures (7 days) of automations, outgoing webhooks and integrations (`/system/failures`);
- applied and pending migrations of the core and of the modules, and compiled modules with the state (enabled or not) of each one (`/system/migrations`).

Retry and discard are recorded in the `system_audit_logs` table (action, user, job, kind and IP). They are operations actions, so they do not go into the business audit (`audit_logs`) nor generate domain events.

## Metrics (Prometheus)

Off by default. To turn it on, set a long, random token:

```bash
METRICS_TOKEN=$(openssl rand -hex 32)
```

`GET /metrics` requires `Authorization: Bearer <METRICS_TOKEN>` (constant-time comparison). Without the variable, the route responds 404; with a wrong token, 401. Scrape example:

```yaml
scrape_configs:
  - job_name: aphel
    metrics_path: /metrics
    authorization:
      type: Bearer
      credentials_file: /etc/prometheus/aphel-token
    static_configs:
      - targets: ['crm.example.com:443']
    scheme: https
```

| Metric | Type | Description |
| --- | --- | --- |
| `aphel_build_info{version}` | gauge | Always 1, with the version |
| `aphel_database_up` | gauge | 1 if the database responded |
| `aphel_jobs{queue,state}` | gauge | Jobs by queue and state (`ready`, `scheduled`, `running`, `failed`) |
| `aphel_jobs_failed_by_kind{kind}` | gauge | Failed jobs by kind |
| `aphel_jobs_oldest_ready_age_seconds` | gauge | Age of the oldest ready job (0 with an empty queue) |
| `aphel_scheduler_overdue_seconds` | gauge | Delay of the most overdue recurring schedule |
| `aphel_workers_alive` | gauge | Workers with a heartbeat within the limit |
| `aphel_worker_heartbeat_age_seconds` | gauge | Seconds since the most recent heartbeat |
| `aphel_http_responses_total{class}` | counter | HTTP responses of this process by class (`2xx`, `4xx`, `5xx`…) |

The queue and worker metrics come from the database (identical on any replica); `aphel_http_responses_total` is per process.

Suggested alerts: `aphel_workers_alive == 0` for 2 min, `aphel_jobs_oldest_ready_age_seconds > 300`, `increase(aphel_jobs{state="failed"}[1h]) > 0`, `rate(aphel_http_responses_total{class="5xx"}[5m]) > 0`.

## Diagnostics: `aphel doctor`

```bash
aphel doctor > aphel-doctor.md
# Docker
docker compose -f docker-compose.prod.yml run --rm app doctor > aphel-doctor.md
```

Prints a Markdown report to attach to issues: version, OS/architecture, MySQL version, state of the services (database, MySQL version, migrations, worker heartbeat, scheduler, queue, storage with a test write, free disk space, outgoing email configured on the screen and reachable over TCP, encryption key, metrics), configuration, pending migrations, compiled extensions with the state of each one and active workers.

The report **does not expose secrets**:

- only a fixed list of non-secret variables appears with its value (port, limits, intervals…);
- `DATABASE_URL` appears without user, password and parameters (`mysql://db:3306/aphel`);
- keys, passwords and tokens (`APP_ENCRYPTION_KEY`, `APP_ENCRYPTION_PREVIOUS_KEYS`, `METRICS_TOKEN`, any `APP_`/`APHEL_`… variable with `KEY`, `SECRET`, `TOKEN`, `PASSWORD` in its name) appear only as `configured`/`not configured`;
- for extra safety, before printing, the text goes through a final sweep: values of sensitive variables (and the user/password extracted from URLs) with 8 or more characters are replaced with `[redacted]` wherever they appear; shorter values are only replaced when they appear as an isolated word in a credential context (right after `:` or `=`, or right before `@`), so that a short password like `aphel` does not erase the word "aphel" from the entire report. Use long passwords in production.

Even so, read the report before publishing it: it shows host names and paths (such as the local storage directory).

## Upgrading: `aphel upgrade`

See [Guided upgrade with `aphel upgrade`](upgrading.md#guided-upgrade-with-aphel-upgrade).
