# Aphel

[![License: AGPL-3.0-or-later](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue.svg)](LICENSE)

**Aphel** is an open source platform for composing business systems from plugins. A small core (identity, records, events, jobs and the plugin runtime) knows no business domain; everything else is a plugin, and a **blueprint** chooses which plugins, configuration and navigation make up a system. The `crm` blueprint, with variants for freelancers, agencies, real estate agencies, consultancies, SaaS organizations and sales teams, is the system Aphel ships today; a blank system is also possible.

Fields, workflow, automations and plugins adapt each installation without changing the code. It runs as **a single binary** (Rust) with the web interface (React) embedded, on top of MySQL. Each installation serves **one company** (the organization): to serve another company, spin up another installation with its own database.

> Current version: **0.1**. See what exists in [Features](#features-of-the-crm-blueprint-in-v01).

## Blueprints

The installer offers a blueprint or a blank system. A blueprint (`blueprints/<key>/blueprint.toml`) lists required and recommended plugins, navigation, roles and configuration packages, and can extend another one. See [the blueprints guide](web/docs/src/content/docs/admin/blueprints.md).

## Features of the `crm` blueprint in v0.1

- **Accounts and access:** first administrator created by the installer, other users by invitation, session sign-in, email verification, forgot password and 2FA.
- **Permissions:** roles (administrator, manager, member and custom), per-module permissions with scope (own, team, organization), teams and sub-teams.
- **Audit:** a log of who changed what and when, with diffs and sensitive data hidden.
- **Contacts and organizations:** complete records, linking people to several organizations, normalized phone numbers (E.164), archiving.
- **Deals:** pipeline with stages, Kanban with drag and drop (and a keyboard "Move to" menu), won and lost with reason, amounts per stage.
- **Tasks:** tasks with due date and assignee, logging of calls and meetings, "My tasks", reminders.
- **Notes and timeline:** Markdown notes with mentions (`@name`), unified timeline per record.
- **Custom fields and saved views:** 14 field types, advanced filters (AND/OR), sorting, columns and personal, team or organization views.
- **Files and attachments:** upload with type checking, authenticated download, local disk or S3-compatible storage.
- **Dashboard:** sales indicators, pending items, recent activity and deals by stage.
- **Notifications:** in-app and by email, with per-kind preferences and a daily digest.
- **Import and export:** CSV import with mapping, validation and reprocessing of errors; export to CSV and XLSX.
- **Installation:** setup wizard on first access, organization settings (name, logo, color, language, time zone, currency).
- **REST API** at `/api/v1`, documented at `/api/docs` (OpenAPI 3.1).

Interface in English and Brazilian Portuguese, responsive (mobile, tablet and desktop), with light and dark themes.

## Installing with Docker

Requirements: Docker with the `docker compose` plugin.

```bash
git clone https://github.com/Viniciusadm/aphel.git aphel && cd aphel
cp .env.example .env
docker compose -f docker-compose.prod.yml up -d
```

Before the last command, change `MYSQL_PASSWORD` and `MYSQL_ROOT_PASSWORD` in `.env`. `up` pulls the prebuilt image (`ghcr.io/viniciusadm/aphel`, amd64 and arm64), without compiling anything on the server. Then open **http://localhost:3001**: the setup wizard registers the company and the first administrator.

Three services start: `app` (web server and API), `worker` (notifications, emails, imports, exports) and `mysql`, with volumes for the database and the files.

Production (HTTPS, reverse proxy, SMTP), manual installation with the binary and systemd: [installation guide](web/docs/src/content/docs/admin/installation.md).

## Configuration

`.env` only holds infrastructure: what the server needs before a database exists, what protects it and what tunes the process. The main ones:

| Variable | Default | What it is for |
| --- | --- | --- |
| `DATABASE_URL` | none | MySQL connection (`mysql://user:password@host:3306/database`). In Docker, built from `MYSQL_*` |
| `APP_ENCRYPTION_KEY` | none | Key (32 bytes in base64) that encrypts the secrets stored in the database. Generate it with `aphel secrets generate-key` |
| `HTTP_HOST` / `HTTP_PORT` | `127.0.0.1` / `3001` | Server address and port (in Docker, `0.0.0.0:3001`) |
| `APP_PORT` | `3001` | Port published on the host (Docker only) |
| `MYSQL_DATABASE`, `MYSQL_USER`, `MYSQL_PASSWORD`, `MYSQL_ROOT_PASSWORD` | `aphel`, required | Database created by compose (Docker only) |
| `RUN_MIGRATIONS` | `true` | Applies the migrations when `serve` and `worker` start |
| `WORKER_CONCURRENCY` | `4` | Parallel jobs per worker |
| `RUST_LOG` | `info` | Log level |

Public address, email sending (SMTP integration), file storage (local disk or S3-compatible), upload limit, webhook delivery and API token limits are configured in the interface (**Settings**), after the installer.

Full list, with the effect of each one: [configuration reference](web/docs/src/content/docs/admin/configuration.md).

## Command line

| Command | What it does |
| --- | --- |
| `aphel serve` | HTTP server (the default without a subcommand). `--with-worker` runs the worker in the same process |
| `aphel worker` | Job queue and scheduler |
| `aphel migrate` | Applies pending migrations and exits |
| `aphel modules list\|doctor\|hooks\|uninstall` | Inspects and uninstalls the modules compiled into the binary |
| `aphel search reindex` | Rebuilds the global search index |
| `aphel secrets generate-key` | Generates a value for `APP_ENCRYPTION_KEY` |
| `aphel upgrade` | Checks requirements, confirms the backup and applies pending migrations |
| `aphel doctor` | Prints a secret-free diagnosis to attach to issues |
| `aphel openapi` | Prints the OpenAPI document |

## Documentation

| Document | Content |
| --- | --- |
| [Installation](web/docs/src/content/docs/admin/installation.md) | Docker, manual installation, reverse proxy, first access |
| [Configuration](web/docs/src/content/docs/admin/configuration.md) | All environment variables |
| [Backup and restore](docs/backup.md) | Scripts, scheduling and tested restore |
| [Upgrading](web/docs/src/content/docs/admin/upgrading.md) | Migrations, upgrading and rolling back |
| [Development](CONTRIBUTING.md) | Local environment, tests, lint and build |
| [Architecture](web/docs/src/content/docs/start/architecture.md) | Crates, layers, request path, records engine, events and jobs |
| [Privacy and LGPD](web/docs/src/content/docs/admin/security.md#privacy-requests) | Personal data, data subject requests, retention and limits |
| [Backend](back/README.md) | Backend layers, API and conventions |

## Development

Requirements: rustup (the Rust version is pinned in `rust-toolchain.toml`), Node.js 24, Docker and GNU Make.

```bash
cp .env.example .env   # fill in APP_ENCRYPTION_KEY with: openssl rand -base64 32
make up
make dev-back
make dev-web
```

`make up` starts MySQL and Mailpit (emails at http://127.0.0.1:8025), `make dev-back` runs the backend at http://127.0.0.1:3001 and `make dev-web` runs Vite at http://localhost:5173. `make lint` runs the CI checks. Details in [CONTRIBUTING.md](CONTRIBUTING.md).

Stack: Rust (axum, sqlx, modular monolith with a kernel and modules), React 19 + TypeScript + Vite, Tailwind CSS, MySQL 8.

## How to contribute

Contributions are welcome: bug reports, suggestions, documentation and code. Read the [contributing guide](CONTRIBUTING.md) and the [code of conduct](CODE_OF_CONDUCT.md). Security vulnerabilities must not be opened as public issues: see [SECURITY.md](SECURITY.md).

## License

Aphel is free software, distributed under the **GNU Affero General Public License v3.0 or later** (SPDX: `AGPL-3.0-or-later`). Full text in [LICENSE](LICENSE).

In short: you can use, study, modify and redistribute Aphel, including commercially. If you distribute a modified version, or offer it for other people to use over a network, you must make the source code of that version available under the same license.
