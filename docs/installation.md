# Installation

Aphel is a single binary (`aphel`) with the frontend embedded. It needs MySQL 8 and a place to store files (local disk or an S3-compatible bucket). There are two paths:

- [Docker Compose](#docker-compose) (recommended): app, worker and MySQL in containers, with persistent volumes.
- [Manual installation](#manual-installation): prebuilt release binary, your own MySQL, systemd and a reverse proxy.

Both paths use artifacts published with every release: the `ghcr.io/viniciusadm/aphel` image and the Linux binaries (x86_64 and aarch64) on the [releases page](https://github.com/Viniciusadm/aphel/releases). Compiling is only needed to change the module selection in `back/aphel.modules.toml` or to run code that is not part of a published release (see [Building from source](#building-from-source)).

After installing, follow the [first access](#first-access) steps. For production, also read [Configuration](configuration.md), [Backup and restore](backup.md) and [Upgrading](upgrading.md).

## Requirements

| Item | Minimum |
| --- | --- |
| CPU and memory | 1 vCPU and 1 GB of RAM for a small team (MySQL included), using the published image or binary. Compiling needs much more: 3 GB of free RAM, about 10 GB of disk and 15 to 40 minutes with 2 vCPUs |
| MySQL | 8.0 or later (the compose file uses 8.4) |
| Docker | A recent Engine with the `docker compose` plugin (Docker path) |
| Rust and Node.js | rustup (the Rust version is pinned in `rust-toolchain.toml`) and Node.js 24, only to [build from source](#building-from-source) |
| SMTP | Optional, but required for invitations, password resets and email notifications |

## Docker Compose

### In 3 commands

```bash
git clone <repository-url> aphel && cd aphel
cp .env.example .env
docker compose -f docker-compose.prod.yml up -d
```

Before the third command, open `.env` and change `MYSQL_PASSWORD` and `MYSQL_ROOT_PASSWORD` (without `@ : / ? # %`, because the password goes into `DATABASE_URL`). `up` pulls the published image; it is only built on the spot if it cannot be pulled.

The default image is `ghcr.io/viniciusadm/aphel:latest` (latest stable release). To pin a version, set in `.env`:

```bash
APHEL_IMAGE=ghcr.io/viniciusadm/aphel:0.2.0
```

Published tags: `X.Y.Z` and `X.Y` for every release, `latest` for the latest stable release and `edge` with the latest commit on `master` (do not use `edge` in production). To build the image locally instead of pulling it, use `docker compose -f docker-compose.prod.yml up -d --build`.

Once the containers are up, open http://localhost:3001 and follow the [first access](#first-access) steps.

### What starts

| Service | What it is |
| --- | --- |
| `app` | `aphel serve`: API, frontend and API documentation, on port `APP_PORT` (default 3001). Applies the migrations on startup |
| `worker` | `aphel worker`: job queue and scheduler (notifications, emails, reminders, imports, exports) |
| `mysql` | MySQL 8.4, with a healthcheck |

`app` and `worker` also have healthchecks, run by the binary itself (the image has no `curl`): `aphel healthcheck` queries the local `/health` and `aphel healthcheck --worker` checks the worker heartbeat in the database. `docker compose ps` shows `healthy` when they are fine; details in [Operations](operations.md#healthcheck-without-curl-aphel-healthcheck).

Volumes: `mysql-data` (database) and `app-storage` (uploaded files, mounted at `/app/storage` in `app` and `worker`). These two are what goes into the [backup](backup.md).

### Useful commands

```bash
docker compose -f docker-compose.prod.yml ps
docker compose -f docker-compose.prod.yml logs -f app worker
docker compose -f docker-compose.prod.yml restart app worker
docker compose -f docker-compose.prod.yml down
docker compose -f docker-compose.prod.yml exec app aphel --help
```

`down` keeps the volumes. `down -v` **deletes the database and the files**.

To avoid typing `-f docker-compose.prod.yml` every time, export `COMPOSE_FILE=docker-compose.prod.yml` in your shell session.

### Production with Docker

- Put a [reverse proxy](#reverse-proxy-and-https) with HTTPS in front and publish the port only on the local interface: `APP_PORT=127.0.0.1:3001` in `.env`.
- Run the installation from the public address already (e.g. `https://crm.yourcompany.com`): the installer saves the browser origin as the public address. It can be edited in **Settings → General**; with `https`, the session cookie automatically becomes `Secure`.
- Configure SMTP in **Settings → Outgoing email** (or in the installer's optional step) and, if you want files off the disk, S3 in **Settings → File storage**. See [Configuration](configuration.md#organization-settings).
- Schedule the [backup](backup.md).

## Manual installation

Example for Debian/Ubuntu with systemd. Adjust paths and names to your distribution.

### 1. Download the binary

Each release publishes `aphel-x86_64-unknown-linux-gnu.tar.gz` and `aphel-aarch64-unknown-linux-gnu.tar.gz`, with a `.sha256` next to them, on the [releases page](https://github.com/Viniciusadm/aphel/releases). The binary is dynamically linked against glibc 2.36 or newer (Debian 12, Ubuntu 23.04 or later) and already embeds the frontend and the migrations. It is the only file needed on the server.

```bash
version=0.2.0
target=x86_64-unknown-linux-gnu
curl -fLO "https://github.com/Viniciusadm/aphel/releases/download/v$version/aphel-$target.tar.gz"
curl -fLO "https://github.com/Viniciusadm/aphel/releases/download/v$version/aphel-$target.tar.gz.sha256"
sha256sum -c "aphel-$target.tar.gz.sha256"
tar -xzf "aphel-$target.tar.gz"
sudo install -m 0755 aphel /usr/local/bin/aphel
aphel --version
```

For a custom module selection or another platform, [build from source](#building-from-source) and install `back/target/release/aphel` the same way.

### 2. Database

In MySQL 8, create the database and the user:

```sql
CREATE DATABASE aphel CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;
CREATE USER 'aphel'@'localhost' IDENTIFIED BY 'change-this-password';
GRANT ALL PRIVILEGES ON aphel.* TO 'aphel'@'localhost';
```

### 3. User, directories and configuration

```bash
sudo useradd --system --home-dir /var/lib/aphel --shell /usr/sbin/nologin aphel
sudo install -d -o aphel -g aphel -m 0750 /var/lib/aphel /var/lib/aphel/storage
sudo install -d -m 0750 /etc/aphel
```

Create `/etc/aphel/aphel.env` (mode `0640`, owner `root:aphel`):

```bash
HTTP_HOST=127.0.0.1
HTTP_PORT=3001
DATABASE_URL=mysql://aphel:change-this-password@127.0.0.1:3306/aphel
APP_ENCRYPTION_KEY=...
RUST_LOG=info
```

Generate the key with `aphel secrets generate-key`. The `.env` holds only infrastructure; the public address, SMTP, upload limit and file storage are configured in the UI after installation. All variables are listed in [Configuration](configuration.md).

The default storage is the local disk at `./storage`, relative to the service's working directory (`/var/lib/aphel/storage` with the `WorkingDirectory` below).

### 4. Migrations

`serve` and `worker` apply the migrations on startup (unless `RUN_MIGRATIONS=false`). To apply them beforehand, manually:

```bash
sudo -u aphel sh -c 'set -a; . /etc/aphel/aphel.env; exec aphel migrate'
```

### 5. systemd services

`/etc/systemd/system/aphel.service`:

```ini
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

`/etc/systemd/system/aphel-worker.service`: the same, with `Description=Aphel CRM (worker)` and `ExecStart=/usr/local/bin/aphel worker`. The worker waits for running jobs to finish when it receives SIGTERM; if your jobs are long (large imports), increase `TimeoutStopSec`.

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now aphel aphel-worker
journalctl -u aphel -u aphel-worker -f
```

In small installations, you can use a single service with `ExecStart=/usr/local/bin/aphel serve --with-worker`.

## Reverse proxy and HTTPS

Aphel does not terminate TLS. In production, put a reverse proxy with HTTPS in front and keep Aphel listening only on `127.0.0.1`. Everything (API, frontend, `/api/docs`) goes through the same port.

Caddy (automatic certificate):

```
crm.yourcompany.com {
    request_body {
        max_size 30MB
    }
    reverse_proxy 127.0.0.1:3001
}
```

Nginx:

```nginx
server {
    listen 443 ssl;
    http2 on;
    server_name crm.yourcompany.com;

    ssl_certificate     /etc/letsencrypt/live/crm.yourcompany.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/crm.yourcompany.com/privkey.pem;

    client_max_body_size 30m;

    location / {
        proxy_pass http://127.0.0.1:3001;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 300s;
    }
}
```

- `client_max_body_size`/`max_size` must be larger than the maximum upload size (Settings → General, default 25 MB).
- The public address (Settings → General) must be the proxy's `https://` address.
- Current limitation: Aphel does not read `X-Forwarded-For` yet, so the audit log and the "forgot password" attempt limit see the proxy's IP, not the user's.

## Building from source

Only needed to change the modules in the build (`back/aphel.modules.toml`, see [Modules](modules/README.md)) or to run a commit without a published release. On a machine with rustup and Node.js 24 (it can be another machine with the same architecture and a compatible glibc):

```bash
git clone https://github.com/Viniciusadm/aphel.git aphel && cd aphel
./build.sh
```

`build.sh` regenerates the module registries, runs `npm ci` and the frontend build in `web/`, and then `cargo build --release`. The result is `back/target/release/aphel`. With Docker, the equivalent is `docker compose -f docker-compose.prod.yml up -d --build`.

The release build is heavy: the first one takes 15 to 40 minutes with 2 vCPUs and uses up to 3 GB of RAM. If possible, compile outside the production server and copy only the binary. Subsequent builds reuse the dependencies in `back/target`, but a change in `aphel-kernel` recompiles all modules.

## First access

1. Open the Aphel address. On a new installation, it opens the **setup wizard** (`/setup`; `/install` redirects to it). It is a single flow with one progress indicator, from the company data to a workspace ready to use.
2. **Company**: language (the wizard switches to it immediately), company name, time zone and currency. Each installation serves a single company (the organization).
3. **Business**: kind of business, what you sell and team size. Asked once: the answers preselect the features and choose the recommended starting template.
4. **Features**: the optional modules, preselected for the business; the ones that connect to external services ask for their settings before being added.
5. **Account**: the first administrator (name, email and password) and a summary of everything chosen. **Create workspace** installs, saves the browser origin as the public address, signs you in and applies the configuration template recommended for the business (module `onboarding`: pipelines, fields, views…), without asking again. Parts of the template that need a feature left off (e.g. automations) are skipped. Until then, the answers survive a page refresh (except the password).
6. **Email** (optional): the SMTP server used for invitations, password resets and notifications. You can do it later: without SMTP, invitations and password resets show a link to copy.
7. **Ready**: shows what was set up and leads to the dashboard. If the template could not be applied, administrators see a "Finish setting up" notice that resumes it at `/setup`, where another template can be chosen; **Settings › Initial assistant › Reopen** does the same later.

To bring the data from another installation, use **Restore from a backup** on the first step of the wizard (before installing): it takes the zip from `aphel org export` and restores the organization, users, records and files. See [Full export](data/org-export-format.md).

Next, in **Settings**, check the public address and the outgoing email, set the organization's logo and color, invite the team and review the pipeline, fields and permissions.

Quick check:

```bash
curl -fsS http://127.0.0.1:3001/health
# or, without curl (inside the container)
docker compose -f docker-compose.prod.yml exec app aphel healthcheck
```

The interactive API documentation is at `/api/docs` and the OpenAPI document at `/api/openapi.json`.

## Demo data

To explore Aphel with fictitious data (organizations, people, deals in every stage, tasks, notes and sample custom fields), use the `demo` subcommand after the installer. It populates the current installation:

```bash
aphel demo seed
aphel demo remove
```

With Docker:

```bash
docker compose -f docker-compose.prod.yml exec app aphel demo seed
docker compose -f docker-compose.prod.yml exec app aphel demo remove
```

`seed` refuses an installation that already has records (unless `--force`) and `remove` deletes only what was created as demo data (tracked in `demo_records`). Even so, do not use it in production.

## Common problems

| Symptom | Likely cause |
| --- | --- |
| Login does not "stick" (goes back to the login screen) | Public address with `https://` (`Secure` cookie) accessed over `http://` on an address that is not `localhost`. Adjust the public address in Settings → General or use HTTPS |
| API routes respond 503 | `DATABASE_URL` missing or database unreachable; check the `app` logs |
| Notifications, imports and exports do not progress | Worker stopped. Check `docker compose -f docker-compose.prod.yml ps` or `systemctl status aphel-worker` |
| Emails do not arrive | Outgoing email not configured (`mail_not_configured`) or with incorrect details; use the test button in Settings → Outgoing email. The error also shows up in the `app` and `worker` logs |
| Upload fails with 413 | File larger than the maximum upload size (Settings → General) or than the reverse proxy limit |
| `MYSQL_PASSWORD` changed in `.env` does not work | The `MYSQL_*` variables only apply when the volume is created. Also change the password in the database with `ALTER USER` |
