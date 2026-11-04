# Configuration

Aphel has two places for configuration:

- **Environment variables** (`.env`): only what is needed before a database exists, what protects the server or what tunes the process (HTTP server, database, encryption key, migrations, logs, worker, health and metrics, webhook protection and demo mode).
- **On-screen settings** (stored in the database, `settings.manage` permission): public address, outgoing email (SMTP), file storage, maximum upload size, webhook delivery, API token limits and community catalog. They take effect without a restart, on the server and on the worker. See [Organization settings](#organization-settings).

## Where the variables come from

- **Process environment.** Always takes precedence.
- **`.env` file.** On startup, the binary looks for a `.env` in the current folder and, if it does not find one, in the folders above it, and loads only the variables that are not yet in the environment.
- **Docker Compose.** In `docker-compose.prod.yml` and in `docker/demo/compose.yml`, the services receive the `.env` from the repository root (`env_file`) and the compose file sets `DATABASE_URL` on top (built from `MYSQL_USER`, `MYSQL_PASSWORD` and `MYSQL_DATABASE`), `HTTP_HOST=0.0.0.0` and `HTTP_PORT=3001`.

There is a single `.env`, at the repository root, copied from [`.env.example`](../.env.example). It serves development (`cargo run` in `back/` finds the root `.env`), the binary in production and the compose files (`docker-compose.prod.yml` and `docker/demo/compose.yml`). The `APP_PORT` and `MYSQL_*` variables are only read by Docker Compose.

Never commit the `.env`: it is already in `.gitignore`.

Boolean values: `0`, `false`, `FALSE`, `no` and `NO` turn off; any other value turns on. Invalid or empty numeric values fall back to the default.

## HTTP server

| Variable | Default | Effect |
| --- | --- | --- |
| `HTTP_HOST` | `127.0.0.1` | Address the server listens on. In Docker it is `0.0.0.0`. An invalid value falls back to the default |
| `HTTP_PORT` | `3001` | Server port. Serves the API (`/api/v1`), the documentation (`/api/docs`), the healthcheck (`/health`) and the embedded frontend |
| `HTTP_TRUSTED_PROXIES` | empty | IPs or CIDR ranges of the trusted reverse proxies, comma-separated (e.g. `127.0.0.1,10.0.0.0/8`). When the connection comes from one of them, the client IP (audit, security events and request limits) becomes the last untrusted address in `X-Forwarded-For`. Empty: the header is ignored and the connection IP is used |
| `RUST_LOG` | `info` | Log filter (`tracing` syntax, e.g. `info,aphel=debug`). Logs go to stderr |

## Database

| Variable | Default | Effect |
| --- | --- | --- |
| `DATABASE_URL` | none | MySQL 8 connection, in the format `mysql://user:password@host:3306/database`. Without it, `aphel serve` starts, but the routes that use the database respond 503; `aphel worker`, `aphel migrate` and `aphel demo` fail. Special characters in the password need URL encoding (`@` becomes `%40`, for example) |
| `RUN_MIGRATIONS` | `true` | `aphel serve` and `aphel worker` apply pending migrations on startup and sync the default data (system fields, pipeline, activity types, loss reasons). When disabled, none of this runs at boot and you must run `aphel migrate` on every upgrade (see [Upgrading](upgrading.md)) |

The pool uses up to 30 connections per process. The database user needs all privileges on the Aphel database (the MySQL image default when using `MYSQL_USER`/`MYSQL_DATABASE`).

## Secret encryption

Secrets stored by Aphel (SMTP password, S3 secret key, webhook signing secrets, integration passwords and keys) are encrypted at rest with AES-256-GCM in the `secrets` table and are never returned by the API after being saved: the screens only show "configured/not configured".

| Variable | Default | Effect |
| --- | --- | --- |
| `APP_ENCRYPTION_KEY` | none | Current key: 32 bytes in base64. Generate it with `aphel secrets generate-key` (or `openssl rand -base64 32`). Without it Aphel works, but saving or reading secrets responds 503 `configuration_error`. An invalid value prevents the server and the worker from starting |
| `APP_ENCRYPTION_PREVIOUS_KEYS` | empty | Old keys, comma-separated, still accepted for decryption during a rotation |

Keep the key outside the database and its backup: whoever has the dump and the key can read the secrets; whoever loses the key loses the secrets (they will have to be entered again). The `app` and the `worker` need the same key.

### Key rotation

1. Generate a new key with `aphel secrets generate-key`.
2. Move the current key to `APP_ENCRYPTION_PREVIOUS_KEYS` and put the new one in `APP_ENCRYPTION_KEY`; restart `app` and `worker`. Old secrets remain readable and those saved from then on use the new key.
3. Run `aphel secrets rotate` (in Docker: `docker compose -f docker-compose.prod.yml exec app aphel secrets rotate`). It re-encrypts with the current key every secret saved with another key and reports how many changed.
4. Remove the old key from `APP_ENCRYPTION_PREVIOUS_KEYS` and restart.

Each secret stores the identifier of the key that encrypted it (the first 16 hexadecimal characters of the key's SHA-256). If a secret was encrypted with a key that is not in either variable, reading it fails with a configuration error.

## Two-factor authentication

Two-step verification (TOTP) stores each user's secret encrypted with `APP_ENCRYPTION_KEY`. Without the key, enabling 2FA responds 503 `configuration_error`; `aphel secrets rotate` also re-encrypts these secrets. The organization policy (Settings → Account security) can require 2FA for administrators or for everyone.

## Login lockout

Consecutive login failures for the same account (email) from the same IP block new attempts for that combination for a while: the API responds 429 `login_locked`, including for emails that do not exist (the response does not reveal whether the account exists). Failures are counted from the last successful login of the same account and IP, so a successful login resets the count. The lockout lasts until `LOGIN_LOCKOUT_WINDOW_SECS` after the last failure and is recorded once in the security events (`login_locked`). It is independent of the per-IP request limit on login.

| Variable | Default | Effect |
| --- | --- | --- |
| `LOGIN_LOCKOUT_ATTEMPTS` | `5` | Failures within the window that lock the account on that IP. `0` disables the lockout |
| `LOGIN_LOCKOUT_WINDOW_SECS` | `900` | Window in which failures are counted and lockout duration (15 minutes). `0` disables the lockout |

## Request limits

Login, SSO discovery, password recovery, verification resend, password change, two-step verification, API tokens and the modules' public routes (incoming webhooks, capture forms, booking, messaging, e-signature, SSO, enrichment and AI, all with `SharedRateLimiter`) count requests in the `rate_limit_counters` table, so the limit applies to the whole installation even with several `aphel serve` processes behind a load balancer. There is no environment variable: the limits are fixed in code, except for the API token limit (Settings → API tokens → Limits).

| Limit | Value | Key |
| --- | --- | --- |
| Login | 10 every 5 min | IP + email |
| SSO discovery | 10 every 5 min | IP |
| Password recovery | 5 every 15 min | IP |
| Email verification resend | 5 every 15 min | user |
| Password change | 5 every 15 min | user |
| Two-step verification | 10 every 15 min | user |
| API tokens | configurable (default 600 every 60 s) | token |
| Incoming webhooks | 120 per minute | webhook URL |

How it works: an approximate sliding window with two counters per key (current and previous window, weighted by the remaining time). Each request costs two short primary-key queries per key (an `INSERT .. ON DUPLICATE KEY UPDATE` and a `SELECT`; a third returns the count when the request is refused). The key is stored only as a SHA-256 hash (IPs and emails do not stay in the table) and rows expire after two windows; cleanup is done by the server itself, in batches, every 512 counted requests. If the database fails or the migration has not run yet, the server logs a warning and falls back to an in-memory counter in the process, without refusing requests because of it.

Login lockout after consecutive failures (above) is a different mechanism, persisted in the security events.

## Authentication emails

Invitation, password reset and email verification are sent **during the request**, through the organization's SMTP, and not through the job queue. This is deliberate: the email carries the token in plain text and the `jobs` table stores the payload unencrypted, so the token never goes to the queue. Consequences:

- The token and the invitation are saved (and the transaction committed) before sending. An SMTP failure undoes nothing and leaves no half-done state: it is recorded in the log (`failed to send email`) and the response is the same as a successful send.
- Without SMTP configured (`mail_not_configured`), the invitation, the invitation resend and an administrator-initiated password reset return the link in the response for the person who performed the action to copy. With SMTP configured but down, the administrator uses "Resend invitation" (which invalidates the previous links) after fixing the server.
- Password recovery requested by the user always responds 202, even when sending fails, so as not to reveal which emails exist.
- The request waits for the SMTP server; the maximum time is that of the configured SMTP transport.

The other emails (notifications, digests, reports, sequences) go through the job queue and are retried in case of failure.

## Worker and scheduler

The worker processes the job queue: domain event dispatch, notifications (in-app and email), task reminders, daily digest, imports, exports and queue cleanup. **Without a running worker, none of this happens.** Run `aphel worker` in a separate process (the production compose file already has the `worker` service) or `aphel serve --with-worker` in small installations.

| Variable | Default | Effect |
| --- | --- | --- |
| `WORKER_CONCURRENCY` | `4` | Jobs processed in parallel per worker process |
| `WORKER_POLL_INTERVAL_MS` | `1000` | Queue polling interval when it is empty |
| `WORKER_JOB_TIMEOUT_SECS` | `300` | Maximum duration of a job; stuck jobs go back to the queue after `timeout + 60 s` |
| `SCHEDULER_INTERVAL_SECS` | `5` | Interval of the recurring job scheduler |

Several workers can run at the same time (the queue uses `FOR UPDATE SKIP LOCKED`).

**Development clock.** In a development build, `AUTOMATIONS_DEV_CLOCK_OFFSET` (e.g. `+2d`) advances only the automation engine's clock (deadlines, `resume_at`, time trigger scan windows, `{{now}}`). The job queue, the scheduler, session and token expiration, computed fields (`days_in_stage`, `is_rotting`) and the listings' relative date filters (`within_last_days`, "today") stay on the database's real clock (`UTC_TIMESTAMP`). This is intentional: the time trigger scan already builds absolute windows from the engine's clock, so it sees the advanced due dates; and shifting the queue would make genuinely scheduled jobs run early. To test something that depends on a relative filter or a computed field, adjust the records' dates instead of the clock.

## Outgoing webhooks

Webhook deliveries are made by the worker. URLs go through SSRF protection on registration and again on every send: the host is resolved and the connection only goes to public addresses (the validated IP is the same one used for the connection, without following redirects or proxies). Details in [Outgoing webhooks](api/webhooks.md).

| Variable | Default | Effect |
| --- | --- | --- |
| `WEBHOOKS_ALLOW_PRIVATE_NETWORKS` | `false` | Allows URLs that resolve to loopback, private networks (RFC 1918, `fc00::/7`), link-local, CGNAT and the like. Enable only in internal installations that need to call systems on their own network |

The https requirement, timeout, automatic deactivation and delivery history retention are in Settings → Webhooks → Delivery.

## Email sync (personal mailboxes)

These apply to the worker (the `email.sync` job, which runs every minute and enqueues one `email.sync.mailbox` per pending mailbox) and to the core IMAP/SMTP provider.

| Variable | Default | Effect |
| --- | --- | --- |
| `EMAIL_SYNC_INTERVAL_MINUTES` | `5` | Minimum interval between two syncs of the same mailbox (1 to 1440) |
| `EMAIL_SYNC_BATCH_SIZE` | `50` | Messages requested from the provider per batch (1 to 500) |
| `EMAIL_SYNC_MAX_BATCHES` | `10` | Batches per job run for a mailbox (1 to 100); the rest is left for the next run |
| `EMAIL_SYNC_MAX_MESSAGE_MB` | `25` | IMAP messages larger than this come in with headers only, without body or attachments (1 to 100) |
| `EMAIL_ALLOW_PRIVATE_NETWORKS` | `false` | Allows IMAP/SMTP servers that resolve to loopback or private networks. Enable in development (local test server) or in internal installations |

The open pixel and click links of tracked emails use the public address saved in Settings → General.

## Monitoring and health

| Variable | Default | Effect |
| --- | --- | --- |
| `METRICS_TOKEN` | — | Enables `/metrics` (Prometheus format), requiring `Authorization: Bearer <token>`. Without it, `/metrics` responds 404 |
| `WORKER_HEARTBEAT_INTERVAL_SECS` | `15` | Interval of the heartbeat written by the worker |
| `HEALTH_WORKER_MAX_SILENCE_SECS` | `90` | A heartbeat older than this puts `/health/ready` at 503 |
| `HEALTH_SCHEDULER_MAX_DELAY_SECS` | `120` | Maximum delay of a recurring schedule |
| `HEALTH_QUEUE_MAX_DELAY_SECS` | `300` | Maximum age of the oldest ready job in the queue |
| `HEALTH_REQUIRE_WORKER` | `true` | With `false`, worker and scheduler are not included in the `/health/ready` result |
| `UPGRADE_MIN_FREE_DISK_MB` | `500` | Minimum free space (local storage and current directory) required by `aphel upgrade`; `0` disables the check |

Details in [`operations.md`](operations.md).

## Demo mode

| Variable | Default | Effect |
| --- | --- | --- |
| `DEMO_MODE` | `false` | Discards emails (including those of the configured SMTP and of the `messaging` module's email channel), shows the demo banner and blocks sign-up, password, 2FA, API tokens and the installer (403 `demo_restricted`) |
| `DEMO_USER_EMAIL` | `demo@aphel.demo` | Visitor user shown in the banner |
| `DEMO_USER_PASSWORD` | `aphel-demo` | Visitor password shown in the banner |
| `DEMO_RESET_TIME` | — | Time (UTC) of the daily reset, display only |

Full environment in [`community/demo.md`](community/demo.md).

## Docker Compose-only variables

Used by `docker-compose.prod.yml`, not by the binary:

| Variable | Default | Effect |
| --- | --- | --- |
| `APP_PORT` | `3001` | Port published on the host |
| `MYSQL_DATABASE` | `aphel` | Name of the database created on MySQL's first boot |
| `MYSQL_USER` | required | Database user |
| `MYSQL_PASSWORD` | required | User password. Avoid `@ : / ? # %`, because it goes into `DATABASE_URL` without encoding |
| `MYSQL_ROOT_PASSWORD` | required | MySQL root password |

The `MYSQL_*` variables only take effect when the MySQL volume is created. Changing the password later also requires changing it in the database (`ALTER USER`).

The development `docker-compose.yml` accepts `MYSQL_PORT`, `MAILPIT_PORT`, `MAILPIT_UI_PORT`, `OBJECT_STORE_PORT` and `OBJECT_STORE_BUCKET` to change host ports (see [Development](development.md)).

## Organization settings

Edited on screen by users with `settings.manage` (API: `GET`/`PATCH /api/v1/organization/settings`). Every change publishes `organization.settings_updated` and writes an audit entry; secrets are never returned through the API, logs or audit (they appear as `"[redacted]"`). The running server and worker start using the new value without a restart (the configuration is reloaded when it changes).

| Setting | Where | Default | Effect |
| --- | --- | --- | --- |
| Public address | General | the origin of the browser that performed the installation | Base for all links in emails (invitation, password reset, notifications, reports, booking, email tracking). With `https://`, the session cookie (`aphel_session`) gets the `Secure` flag. Links never use the request's `Host` header |
| Maximum upload size | General | 25 MB | Limit for each upload. Also adjust the reverse proxy limit |
| Outgoing email (SMTP) | Email | not configured (in a development build the form is prefilled with Mailpit: `127.0.0.1:1025`, without TLS) | Server used for all system emails. The password is kept in the secret vault (requires `APP_ENCRYPTION_KEY`). The test button sends an email to the user themselves and shows the server error (authentication, TLS, connection) |
| File storage | Storage | local disk at `./storage` (in the Docker image, `/app/storage`, volume `app-storage`) | Local disk or S3-compatible (bucket, region, endpoint for MinIO/SeaweedFS/R2/B2, access key, `http` only for local S3, path-style addressing). The secret key is kept in the vault. "Test connection" writes, reads and deletes an object. Changing the driver does not copy existing files |
| Webhook delivery | Webhooks → Delivery | no https requirement (the installer enables it when the public address is https), 10 s, deactivates after 15 failures, 30-day history | `https` requirement, maximum time for each delivery (1 to 60 s), consecutive failures before deactivating the endpoint and days of history |
| API token limits | API tokens → Limits | 600 requests every 60 s | Sliding window per token (`X-RateLimit-*`, 429 when exceeded) |
| Community catalog | Integrations → Modules → Catalog | embedded catalog | Embedded (no internet), remote `https` index (http only with the development option) or disabled; maximum fetch time |

Without SMTP configured nothing breaks: sending fails with `mail_not_configured` (recorded in the job), the invitation and an administrator-initiated password reset show the link to copy, and the settings screen warns until SMTP is configured. The installer has an optional step to configure outgoing email.

Downloads always go through Aphel (with permission checks), including on S3: the bucket does not need to be and must not be public. `serve` and `worker` need to see the same storage (exports are written by the worker and downloaded by the server). Custom object limits are fixed (50 objects, 200 fields per object).

## Environments

Keep one `.env` per environment (development, staging, production), outside the repository, or inject the variables through the orchestrator (systemd, Docker, Kubernetes). Things that change between environments:

- `DATABASE_URL` and `APP_ENCRYPTION_KEY` specific to each environment.
- Public address, SMTP and storage configured on each environment's screen (they live in each one's database).
- Separate database and bucket: never point staging at production data.

## Core SMTP transport

`GET`/`PATCH /api/v1/organization/settings` expose core `mail` settings (`host`, `port`, `from`, `username`, `security`: `starttls`, `tls` or `none`). The outgoing-email settings page is available without the integrations plugin. `mail_password` is write-only: omit it to preserve the vault secret, send a value to replace it or `null` to remove it. Responses contain `mail_secret_configured`, never the password.

When no core SMTP host is configured, infrastructure can supply `SMTP_HOST`, `SMTP_PORT`, `SMTP_FROM`, `SMTP_USERNAME`, `SMTP_PASSWORD` and `SMTP_SECURITY`. An installation without a configured transport reports `mail_not_configured`.
