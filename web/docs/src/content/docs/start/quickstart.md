---
title: Quickstart
description: Get a running Aphel instance with Docker Compose or from source, run the setup wizard and find your way around.
sidebar:
  order: 2
---

This page gets you from nothing to a signed-in instance. Choose Docker if you want to run Aphel, or the source build if you want to work on it. For production hardening (HTTPS, reverse proxy, SMTP, systemd) see [Installation](/_aphel/docs/admin/installation/).

## Option 1: Docker Compose

Requirements: Docker with the `docker compose` plugin.

1. Get the repository and create your environment file.

   ```bash
   git clone https://github.com/Viniciusadm/aphel.git aphel && cd aphel
   cp .env.example .env
   ```

2. Edit `.env` and set these values:

   | Variable | What to set |
   | --- | --- |
   | `MYSQL_PASSWORD` | A new password for the application database user. |
   | `MYSQL_ROOT_PASSWORD` | A new password for the MySQL root user. |
   | `APP_ENCRYPTION_KEY` | A 32-byte key in base64. It encrypts secrets stored in the database. |

   Avoid the characters `@ : / ? # %` in the MySQL passwords, because the password becomes part of the database URL. To generate the encryption key and write it into `.env` on Linux:

   ```bash
   sed -i "s|^APP_ENCRYPTION_KEY=.*|APP_ENCRYPTION_KEY=$(openssl rand -base64 32)|" .env
   ```

   Keep the key outside your database backups. Without it, saving secrets such as an SMTP password fails, and secrets already stored cannot be decrypted.

3. Start the stack.

   ```bash
   docker compose -f docker-compose.prod.yml up -d
   ```

   This pulls the prebuilt image (`ghcr.io/viniciusadm/aphel`) and starts three services: `app` (web server and API), `worker` (notifications, emails, imports, exports and scheduled work) and `mysql`. Database migrations run automatically when the services start.

4. Check that it is healthy.

   ```bash
   docker compose -f docker-compose.prod.yml ps
   curl http://localhost:3001/_aphel/health
   ```

   The `app` service listens on port `3001` unless you set `APP_PORT` in `.env`.

5. Open **http://localhost:3001** in a browser and continue with [the setup wizard](#the-setup-wizard).

## Option 2: From source

Requirements: rustup (the Rust version is pinned by the repository), Node.js 24, Docker and GNU Make.

1. Create the environment file and set the encryption key.

   ```bash
   cp .env.example .env
   sed -i "s|^APP_ENCRYPTION_KEY=.*|APP_ENCRYPTION_KEY=$(openssl rand -base64 32)|" .env
   ```

   The defaults in `.env.example` already point to the development database started in the next step.

2. Start MySQL and Mailpit (a local mail catcher).

   ```bash
   make up
   ```

   Mailpit shows outgoing email at http://127.0.0.1:8025.

3. Run the backend. This generates the module registries, builds the web application if `web/dist` is missing, and starts the server on http://127.0.0.1:3001.

   ```bash
   make dev-back
   ```

4. In a second terminal, run the web development server. It proxies `/api` to the backend.

   ```bash
   make dev-web
   ```

   Open **http://localhost:5173** and continue with [the setup wizard](#the-setup-wizard).

`make dev-back` runs only the HTTP server. Jobs such as emails, imports and exports are processed by the worker. To run it in the same process, start the backend with:

```bash
cd back && cargo run -- serve --with-worker
```

To build a single release binary with the web application embedded:

```bash
make build
```

The binary is written to `back/target/release/aphel`. Run it from the repository root so it finds `.env`:

```bash
back/target/release/aphel serve --with-worker
```

## The setup wizard

On a new database, the first visit shows the setup wizard. It is available only until the first company is created. Afterwards it answers 404 and the normal sign-in page takes over.

| Step | What you provide |
| --- | --- |
| Company | Company name, language, time zone and default currency. |
| Business | The kind of business: freelancer, agency, real estate, consultancy, software or SaaS, B2B sales, or other. This selects a [blueprint](/_aphel/docs/admin/blueprints/). You can instead choose **Start with a blank system**. |
| Features | The modules to enable. The blueprint preselects its recommended ones, and the dependencies of anything you pick are added automatically. |
| Account | The first administrator: name, email and a password of 8 to 128 characters. |
| Email | Optional SMTP settings. You can skip this step. |
| Ready | A summary and suggested next steps. |

Notes:

- Open the wizard at the address your users will use. The installer saves the origin of your browser as the public address, which Aphel uses for links in emails.
- You can choose **Restore from a backup instead** on the first step to restore a backup exported from another installation. See [Backup and restore](/_aphel/docs/admin/backup-and-restore/).
- Without SMTP, invitations and password resets show a link for you to copy and send. You can configure email later in **Settings**, under **Outgoing email**.
- Your answers are kept as a draft in the browser while you move between steps. The password is never kept.

## What you see first

After you finish, you are signed in as the administrator. The home page depends on the blueprint: the `crm` family opens the CRM home, and a blank system shows an empty page until you enable modules.

- The sidebar holds the navigation of the enabled modules. Each module contributes its own entries.
- **Settings** is where administrators manage users, roles, teams, fields, modules, plugins, API tokens, webhooks, integrations and the audit log.
- The ready screen suggests three next steps: import people and organizations from a spreadsheet, invite your team from **Settings**, **Members**, and adjust fields, workflow and features.

## Where to go next

- Learn the vocabulary in [Concepts](/_aphel/docs/start/concepts/).
- Explore day-to-day features in [Using Aphel](/_aphel/docs/using/overview/).
- Invite people and set up access in [Users and teams](/_aphel/docs/admin/users-and-teams/) and [Roles and permissions](/_aphel/docs/admin/roles-and-permissions/).
- Plan a production deployment with [Installation](/_aphel/docs/admin/installation/), [Configuration](/_aphel/docs/admin/configuration/) and [Backup and restore](/_aphel/docs/admin/backup-and-restore/).
- Build your own extension with [Your first plugin](/_aphel/docs/plugins/first-plugin/).
