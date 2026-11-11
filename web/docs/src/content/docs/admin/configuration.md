---
title: Configuration
description: Every environment variable the aphel binary reads, the settings edited in the app, and which one wins when both exist.
sidebar:
  order: 2
---

Aphel has two layers of configuration:

- **Environment variables**, read by the binary when it starts. They hold what the server needs before a database exists, what protects it, and what tunes the process.
- **Organization settings**, stored in the database and edited in the app (or through the API). They apply without restarting the server or the worker.

The binary loads a `.env` file from the current directory or a parent directory, if one exists. Variables already present in the process environment are not overridden by the file. Restart `aphel serve` and `aphel worker` after changing an environment variable.

## Environment variables

Boolean variables accept `0`, `false`, `no` and `off` as false unless the table says otherwise. Numeric variables that cannot be parsed fall back to their default.

### Server and database

| Variable | Default | Meaning |
| --- | --- | --- |
| `HTTP_HOST` | `127.0.0.1` | IP address the server binds to. An invalid value falls back to the default. The Docker image sets `0.0.0.0`. |
| `HTTP_PORT` | `3001` | TCP port. |
| `DATABASE_URL` | none | MySQL connection URL, `mysql://user:password@host:port/database`. Without it the server still starts, but every endpoint that needs the database fails with a configuration error. Commands such as `migrate`, `worker` and `upgrade` refuse to run. |
| `CONTROL_DATABASE_URL` | none | MySQL connection URL of the control database, which lists the tenants of a multi-tenant installation. Mutually exclusive with `DATABASE_URL`. Every tenant database lives on the same server, with the same credentials and its own database name. `serve` routes requests by host to the tenants of the control database, `worker` (or `serve --with-worker`) runs the jobs and schedules of every active tenant, and `migrate` migrates the control database and every tenant. Tenants are managed with `aphel tenants` (the credentials need `CREATE` and `DROP`); the operator console is not available in this build yet. |
| `CONSOLE_HOST` | none | Host name of the operator console in a multi-tenant installation. Requires `CONTROL_DATABASE_URL`. |
| `TENANT_DATABASE_MAX_CONNECTIONS` | `10` | Maximum connections of each tenant's database pool in a multi-tenant installation. |
| `RUN_MIGRATIONS` | `true` | Apply pending migrations when `serve` or `worker` starts. Set to a false value to run them only through `aphel migrate` or `aphel upgrade`. |
| `RUST_LOG` | `info` | Log filter in `tracing` syntax. Logs go to standard error. |
| `HTTP_TRUSTED_PROXIES` | empty | Comma-separated list of IP addresses or CIDR networks (for example `10.0.0.0/8, 192.168.1.5`) of reverse proxies. When the direct peer is in the list, the client address is taken from `X-Forwarded-For`. Invalid entries are ignored with a warning. |

### Secrets

| Variable | Default | Meaning |
| --- | --- | --- |
| `APP_ENCRYPTION_KEY` | none | Base64 encoding of exactly 32 random bytes. Encrypts every stored secret with AES-256-GCM. Without it, saving secrets fails and the server logs a warning at start. Generate with `aphel secrets generate-key`. |
| `APP_ENCRYPTION_PREVIOUS_KEYS` | empty | Comma-separated list of former keys, in the same format. Values encrypted with them stay readable during a key rotation. See [Security](/_aphel/docs/admin/security/). |

### Worker and scheduler

| Variable | Default | Meaning |
| --- | --- | --- |
| `WORKER_CONCURRENCY` | `4` | Jobs a worker runs at the same time. Must be greater than 0. In a multi-tenant installation the budget is shared by every tenant of the process, handed out in turn to the tenants with pending jobs. |
| `WORKER_POLL_INTERVAL_MS` | `1000` | How often the worker looks for ready jobs, in milliseconds. |
| `WORKER_JOB_TIMEOUT_SECS` | `300` | Time a job may run. A job that holds its lock longer than this plus 60 seconds is considered lost and is released for another attempt, or failed when attempts are exhausted. |
| `SCHEDULER_INTERVAL_SECS` | `5` | How often the scheduler checks for due recurring schedules. |
| `WORKER_HEARTBEAT_INTERVAL_SECS` | `15` | How often a worker records that it is alive. Must be greater than 0. |

### Health, metrics and upgrades

| Variable | Default | Meaning |
| --- | --- | --- |
| `HEALTH_REQUIRE_WORKER` | `true` | When true, `/_aphel/health/ready` fails if no recent worker heartbeat exists. Set to `0`, `false`, `no` (any case) for installations where the worker check should be skipped. |
| `HEALTH_WORKER_MAX_SILENCE_SECS` | `90` | Maximum age of the last worker heartbeat. Also used by `aphel healthcheck --worker`. |
| `HEALTH_SCHEDULER_MAX_DELAY_SECS` | `120` | How late the most overdue recurring schedule may be. |
| `HEALTH_QUEUE_MAX_DELAY_SECS` | `300` | How long the oldest ready job may wait. |
| `METRICS_TOKEN` | none | Enables `GET /_aphel/metrics` and is the bearer token it requires. Without it the endpoint answers `404`. |
| `UPGRADE_MIN_FREE_DISK_MB` | `500` | Minimum free disk space that `aphel upgrade` and `aphel doctor` require on the file storage folder and the working directory. |

### Outbound traffic

| Variable | Default | Meaning |
| --- | --- | --- |
| `WEBHOOKS_ALLOW_PRIVATE_NETWORKS` | `false` | Allows outgoing webhook deliveries to loopback and private-network addresses. Keep it off in production: it is the protection against server-side request forgery. |

### Sign-in protection

| Variable | Default | Meaning |
| --- | --- | --- |
| `LOGIN_LOCKOUT_ATTEMPTS` | `5` | Failed sign-ins within the window that lock the account. `0` disables the lockout. |
| `LOGIN_LOCKOUT_WINDOW_SECS` | `900` | Length of the window, in seconds. `0` disables the lockout. |

### Outgoing email fallback

These variables are used only when the matching value is not set in Settings, Outgoing email. See [which setting wins](#which-setting-wins).

| Variable | Default | Meaning |
| --- | --- | --- |
| `SMTP_HOST` | none | SMTP server. |
| `SMTP_FROM` | none | Sender address. Email is considered configured only when both a host and a sender exist. |
| `SMTP_PORT` | `587` | SMTP port. |
| `SMTP_SECURITY` | `starttls` | `starttls`, `tls` or `none`. Any other value means `starttls`. |
| `SMTP_USERNAME` | none | SMTP user name. |
| `SMTP_PASSWORD` | none | SMTP password, used when no password is stored in the secret vault. |

### Email module

These are read by the email module. They do not apply when the module is not compiled in.

| Variable | Default | Meaning |
| --- | --- | --- |
| `EMAIL_SYNC_INTERVAL_MINUTES` | `5` | How often each mailbox is synchronized. Accepted range 1 to 1440. |
| `EMAIL_SYNC_BATCH_SIZE` | `50` | Messages fetched per batch. Accepted range 1 to 500. |
| `EMAIL_SYNC_MAX_BATCHES` | `10` | Batches per synchronization run. Accepted range 1 to 100. |
| `EMAIL_SYNC_MAX_MESSAGE_MB` | `25` | Largest message that is downloaded. Accepted range 1 to 100. |
| `EMAIL_ALLOW_PRIVATE_NETWORKS` | `false` | Allows IMAP and SMTP mailboxes on private-network addresses. Only `1`, `true`, `TRUE` and `yes` enable it. |

### Demo mode

| Variable | Default | Meaning |
| --- | --- | --- |
| `DEMO_MODE` | `false` | Turns the installation into a public demonstration: outgoing email is discarded instead of sent, and password reset, sign out of all devices, password change, session revocation, API tokens and two-factor changes answer `403 demo_restricted`. The installer is also blocked. Only `1`, `true`, `TRUE` and `yes` enable it. |
| `DEMO_USER_EMAIL` | `demo@aphel.demo` | Email of the limited visitor account created by `aphel demo seed` and `aphel demo reset`. |
| `DEMO_USER_PASSWORD` | `aphel-demo` | Password of that visitor. |
| `DEMO_RESET_TIME` | `03:00` | Daily reset time, in UTC. |

### Variables for Docker Compose only

The binary ignores these. `docker-compose.prod.yml` uses them.

| Variable | Default | Meaning |
| --- | --- | --- |
| `APHEL_IMAGE` | `ghcr.io/viniciusadm/aphel:latest` | Image for the `app` and `worker` services. |
| `APP_PORT` | `3001` | Port published on the host. |
| `MYSQL_DATABASE` | `aphel` | Database name. |
| `MYSQL_USER`, `MYSQL_PASSWORD` | required | Application database user. |
| `MYSQL_ROOT_PASSWORD` | required | MySQL root password. |

:::note
`HOSTNAME`, when set, identifies the host of a worker in its heartbeat. `aphel healthcheck --worker` then checks only the worker of the same host.
:::

## Organization settings

Settings that live in the database are edited under Settings in the app. They require the `settings.manage` permission and take effect without a restart. The same data is available at `GET` and `PATCH /api/v1/organization/settings`.

| Screen | Setting | Default | Notes |
| --- | --- | --- | --- |
| General | Public address | `http://localhost:5173` until set | The base of every link in email (invitations, password resets, notifications). When it starts with `https://`, the session cookie is marked `Secure`. The installer fills it with the address your browser used. |
| General | App path | `/` | Where the internal app is served: `/` or a single segment such as `/app` (lowercase letters, digits, hyphens; not `/api` or `/_aphel`). With a segment, the other paths stay free for a public site and `/` redirects to the app. Links in email use the public address plus this path. Changing it also requires `system.manage`. The app always answers at `/_aphel/app/` too, and `aphel mounts reset` moves it back to `/`. |
| Site | Site domain | none | A host name such as `www.example.com`, optionally with a port, pointed at this server. On that host, every path outside `/api` and `/_aphel` belongs to the site. Changing it requires `system.manage`. |
| Site | Site path | `/s` | Where plugin site pages answer: `/` or one segment. Without a site domain it must differ from the app path; the longer path wins, so the app at `/app` and the site at `/` share the address. Changing it requires `system.manage`. |
| Site | Let search engines find the site | on | Off, site pages carry `noindex`, `robots.txt` disallows everything and the sitemap answers 404. |
| Site | Theme | Default theme | A theme from an installed plugin (`[site.theme]`) or the built-in one. |
| Site | Tagline | none | Up to 160 characters, shown below the company name. Name, logo and color come from the organization. |
| Site | Menu | Declaration order, all shown | Order and visibility of the menu items that plugins contribute, plus links added by hand (up to 20): a site path such as `/book/intro-call` or an address starting with `https://`. |
| Site | Languages | Only the organization language | Up to five extra languages for the site, each under its own code (`/en`, `/es`). |
| Site | Create a contact in People | off | With the `people` module, links each customer with a confirmed email to the person with the same email, creating one for the chosen owner when none exists. |
| General | Maximum upload size | 25 MB | Between 1 and 4096 MB. Applies to attachments, imports and emails. Also raise the proxy limit. |
| File storage | Type | Local disk, folder `./storage` | `Local disk` or `S3 compatible`. The folder is relative to the working directory of the server and the worker, which must see the same folder. |
| File storage | S3 options | bucket `aphel`, region `us-east-1`, path-style on, plain http off | Bucket, region, optional endpoint (MinIO, SeaweedFS, R2, B2), access key, secret key. The secret key goes to the secret vault. A test button writes, reads and deletes a small object. |
| Outgoing email | SMTP | none | Host, port (587), security (`starttls`), user name, sender, password. The password goes to the secret vault and is never shown. Host and sender must be set together. |
| Outgoing webhooks, Delivery | Require https | off | Rejects plain `http` endpoint URLs. |
| Outgoing webhooks, Delivery | Maximum time per delivery | 10 s | 1 to 120. |
| Outgoing webhooks, Delivery | Consecutive failures before disabling | 15 | 1 to 1000. |
| Outgoing webhooks, Delivery | Days of delivery history | 30 | 1 to 3650. |
| API tokens | Requests per window | 600 | 1 to 100000, per token. |
| API tokens | Window | 60 s | 1 to 3600. |
| Community catalog | Source | Built-in catalog | `builtin` (no internet), `remote` (with an index URL) or `off`. A remote index must be an https URL unless the development option allows http. Fetch time is 1 to 120 s, default 10. |

The Site screen exists only when the `site` module is enabled. It also previews a theme before it goes live (only in the browser that opens the preview, for 30 minutes), manages redirects added by hand (an old path to a site path or another website, permanent or temporary), lists the redirects of moved addresses (remove one to make the old address answer 404 again) and refreshes every page on demand.

Changing the storage type does not copy existing files: they stay in the previous storage. Move them yourself before switching. The secret values in these settings are written to the audit log as `[redacted]`.

The Webhooks and Catalog screens exist only when the corresponding modules are compiled in and enabled.

## Which setting wins

| Topic | Rule |
| --- | --- |
| Public address, app path, site path and domain, upload limit, file storage, API token limits, webhook delivery, catalog | Only the organization setting exists. There is no environment equivalent. |
| Outgoing email (SMTP) | Per value, the organization setting wins when its SMTP host is set. A value left empty there falls back to the matching `SMTP_*` variable. When no host is set in the app, everything comes from the environment. The password is read from the secret vault first, then from `SMTP_PASSWORD`. |
| Private-network webhooks | The environment variable `WEBHOOKS_ALLOW_PRIVATE_NETWORKS` decides. The Delivery screen cannot override it. |
| Worker, scheduler, health, lockout, email synchronization | Only the environment. |
| Module settings | Stored per module and edited on the Modules screen. See [Modules](/_aphel/docs/admin/modules/). |

With no outgoing email configured, Aphel keeps working: invitation and password-reset responses include a link for the administrator to copy and hand over.

## Check the result

`aphel doctor` prints a diagnostic report of the configuration without revealing secrets. See [CLI](/_aphel/docs/admin/cli/) and [Operations](/_aphel/docs/admin/operations/).
