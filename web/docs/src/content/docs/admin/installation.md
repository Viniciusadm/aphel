---
title: Installation
description: Install Aphel with Docker Compose or from source, prepare MySQL and complete the first-run installer.
sidebar:
  order: 1
---

Aphel is one binary, `aphel`, plus a MySQL database. The binary serves the web app and the API, and a second process of the same binary (the worker) runs background jobs. This page covers both supported ways to run it and the first-run installer.

## Requirements

| Item | Requirement |
| --- | --- |
| Database | MySQL 8.0 or newer. MariaDB is rejected by `aphel doctor` and `aphel upgrade`. |
| Processes | One `aphel serve` and one `aphel worker`, or a single `aphel serve --with-worker`. |
| Disk | Space for uploaded files (local disk, unless you use S3) and at least the free space set by `UPGRADE_MIN_FREE_DISK_MB` (default 500 MB) for upgrades. |
| Source build only | Rust 1.99.0 (pinned in `rust-toolchain.toml`), Node.js 24 and npm. |
| Secrets | A random 32-byte key in `APP_ENCRYPTION_KEY`. Without it, no secret (SMTP password, S3 key, integration credentials, two-factor secrets) can be saved. |

The web app is embedded in the binary when it is compiled, so there is no separate front-end to deploy.

## Install with Docker Compose

The repository ships `docker-compose.prod.yml` with three services:

| Service | Image and command | Role |
| --- | --- | --- |
| `app` | `ghcr.io/viniciusadm/aphel:latest` (override with `APHEL_IMAGE`), runs `serve` | HTTP server on port 3001 inside the container, published on `APP_PORT` (default 3001) |
| `worker` | same image, runs `worker` | Background jobs and the scheduler |
| `mysql` | `mysql:8.4` | Database, with the `mysql-data` volume |

The `app` and `worker` services share the `app-storage` volume, mounted at `/app/storage`. The image runs as the user `aphel` (UID 10001).

1. Copy the example environment file and edit it.

   ```bash
   cp .env.example .env
   ```

2. Set at least these values in `.env`: `MYSQL_USER`, `MYSQL_PASSWORD`, `MYSQL_ROOT_PASSWORD` (Compose refuses to start without them), optionally `MYSQL_DATABASE` (default `aphel`) and `APP_PORT`, and `APP_ENCRYPTION_KEY`. Avoid the characters `@ : / ? # %` in the MySQL password, because Compose places it inside `DATABASE_URL`.

3. Generate the encryption key and paste it into `.env`.

   ```bash
   openssl rand -base64 32
   ```

   `aphel secrets generate-key` prints an equivalent value.

4. In production, pin the image to a version instead of `latest` by setting `APHEL_IMAGE` in `.env`.

5. Start the stack.

   ```bash
   docker compose -f docker-compose.prod.yml up -d
   ```

The `app` container applies pending database migrations when it starts (`RUN_MIGRATIONS` defaults to `true`). The `app` health check runs `aphel healthcheck` and the `worker` health check runs `aphel healthcheck --worker`. The worker starts only after the `app` is healthy.

Open `http://localhost:3001/setup` (or your own address) to run the [first-run installer](#the-first-run-installer).

:::note
`docker-compose.prod.yml` overrides `DATABASE_URL`, `HTTP_HOST` and `HTTP_PORT` for the containers. Everything else in `.env` reaches the containers through `env_file`. See [Configuration](/_aphel/docs/admin/configuration/) for every variable.
:::

## Install from source

1. Create the database and a user (see [Prepare MySQL](#prepare-mysql)).

2. Copy `.env.example` to `.env` in the repository root and set `DATABASE_URL` and `APP_ENCRYPTION_KEY`. The binary looks for `.env` in the current directory and in its parent directories.

3. Build everything with the build script. It syncs the module registries, installs and builds the web app, then compiles the release binary.

   ```bash
   ./build.sh
   ```

   `make build` runs the same script after `make modules`.

4. The binary is `back/target/release/aphel`. Run the server and the worker from the directory that contains `.env`.

   ```bash
   back/target/release/aphel serve
   back/target/release/aphel worker
   ```

   For a small installation you can run one process instead: `aphel serve --with-worker`.

Run with no subcommand, `aphel` behaves as `aphel serve`.

### Run as services

The repository does not ship service files. The backup scripts document this layout, which you can reproduce with systemd: the binary and scripts in `/opt/aphel/`, environment in `/etc/aphel/aphel.env`, files in `/var/lib/aphel/storage`, a system user `aphel`, and two units named `aphel` and `aphel-worker`.

```ini
[Unit]
Description=Aphel server
After=network-online.target mysql.service

[Service]
User=aphel
WorkingDirectory=/var/lib/aphel
EnvironmentFile=/etc/aphel/aphel.env
ExecStart=/opt/aphel/aphel serve
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

The worker unit is identical except for `Description` and `ExecStart=/opt/aphel/aphel worker`. In the settings screen, set the file storage folder to the same absolute path for both processes. Both processes must see the same folder.

### Behind a reverse proxy

- Terminate TLS at the proxy and forward to `HTTP_HOST:HTTP_PORT`.
- Set the public address (Settings, General) to the `https://` address people use. With an `https` address the session cookie is marked `Secure`.
- Set `HTTP_TRUSTED_PROXIES` to the proxy address or network. Aphel uses the `X-Forwarded-For` header only when the direct peer is in that list. Without it, every request appears to come from the proxy, which affects the audit log, rate limits and sign-in lockout.
- Raise the proxy body-size limit to match the upload limit (Settings, General).

## Prepare MySQL

Docker Compose creates the database and user from `MYSQL_DATABASE`, `MYSQL_USER` and `MYSQL_PASSWORD`. For your own MySQL server, create an empty database and a user with full rights on it.

```sql
CREATE DATABASE aphel CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;
CREATE USER 'aphel'@'localhost' IDENTIFIED BY 'a-strong-password';
GRANT ALL PRIVILEGES ON aphel.* TO 'aphel'@'localhost';
```

Then point `DATABASE_URL` at it.

```bash
DATABASE_URL=mysql://aphel:a-strong-password@127.0.0.1:3306/aphel
```

Rights on the Aphel database are enough. `aphel db reset` also needs `DROP` and `CREATE`, which `ALL PRIVILEGES` on the database provides.

## The first-run installer

On a fresh database, the sign-in page redirects to the installer at `/setup` (the path `/install` redirects there too). The installer is available only until the organization is created. Afterwards its API answers `404`. The wizard stores a draft in the browser (without the password), so you can reload without losing your answers.

| Step | What you provide |
| --- | --- |
| Organization | Organization name, time zone, currency and language. A link on this step switches to restoring a backup instead. |
| Business | The blueprint (business type), what you offer and your team size. Shown only when the build contains blueprints. See [Blueprints](/_aphel/docs/admin/blueprints/). |
| Features | The optional modules to turn on, with the settings each one needs. The blueprint pre-selects its recommended modules that need no configuration, and always installs its required ones. Dependencies are added automatically. See [Modules](/_aphel/docs/admin/modules/). |
| Account | Name, email and password of the first administrator (8 to 128 characters). |
| Email | Optional outgoing email (SMTP). You can skip it and set it up later. |
| Ready | Result of the setup, including applying the blueprint's configuration packages. |

When you finish, Aphel creates the organization and the administrator (with a verified email) in one transaction, enables the chosen modules, applies the blueprint configuration, signs you in and shows the app. The address your browser used becomes the public address. You can change it later.

:::caution
The installer is open to anyone who can reach the server until the first installation completes. Complete it before exposing the instance on a public address.
:::

### Restore a backup instead

Choose the restore link on the first step to upload a full export (`aphel.organization_export` zip, produced by `aphel org export` or the Full export screen). The restore creates the organization and its members with their passwords, installs the configuration and restores records, history and files. Then sign in with an administrator from the backup.

- The upload is limited to 2 GiB. For larger archives use `aphel org import FILE` on the server.
- The route accepts 5 attempts per hour per IP address and 20 per hour in total.
- It works only on an installation that is not set up yet.

See [Backup and restore](/_aphel/docs/admin/backup-and-restore/).

### Start over

To discard everything and run the installer again on a test installation, stop all servers and workers and run:

```bash
aphel db reset --confirm aphel
```

The value after `--confirm` must be the exact database name from `DATABASE_URL`. Stored files are not deleted. See [CLI](/_aphel/docs/admin/cli/).

## After installing

1. Check `aphel doctor` or the System screen: the database, storage, worker and scheduler checks should be green.
2. Configure the public address, file storage and outgoing email in [Configuration](/_aphel/docs/admin/configuration/).
3. Turn on two-factor authentication for administrators: see [Security](/_aphel/docs/admin/security/).
4. Schedule backups: see [Backup and restore](/_aphel/docs/admin/backup-and-restore/).
