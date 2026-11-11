---
title: CLI
description: Every subcommand and flag of the aphel binary, with examples.
sidebar:
  order: 13
---

The `aphel` binary is both the server and the administration tool. Run `aphel --help` for the list of subcommands, `aphel <command> --help` for the flags of one, and `aphel --version` for the version.

Running `aphel` with no subcommand is the same as `aphel serve`.

Except where noted, every command reads the same configuration as the server: environment variables and the `.env` file (see [Configuration](/_aphel/docs/admin/configuration/)). Commands exit with status 0 on success and a non-zero status on failure. Output and logs go to the terminal, and logs follow `RUST_LOG`.

In Docker, run a command in a one-off container with the `app` service, whose entrypoint is `aphel`:

```bash
docker compose -f docker-compose.prod.yml run --rm --no-deps app doctor
```

Or run it in the running container:

```bash
docker compose -f docker-compose.prod.yml exec app aphel doctor
```

## Summary

| Command | Purpose |
| --- | --- |
| `serve` | Start the HTTP server |
| `worker` | Run the job worker and scheduler |
| `migrate` | Apply pending database migrations |
| `upgrade` | Check requirements, confirm a backup and apply migrations |
| `doctor` | Print a sanitized diagnostic report |
| `healthcheck` | Exit 0 when the server or the worker is healthy |
| `secrets generate-key` | Print a new encryption key |
| `secrets rotate` | Re-encrypt stored secrets with the current key |
| `db reset` | Delete and recreate the database |
| `tenants list`, `create`, `suspend`, `resume`, `delete`, `migrate` | Manage the tenants of a multi-tenant installation |
| `console create-operator` | Create an operator of the multi-tenant console |
| `tenancy-check` | Check that two throwaway tenants cannot reach each other |
| `modules list`, `modules doctor`, `modules uninstall` | Inspect and remove modules |
| `mounts reset` | Serve the internal app at `/` again |
| `org export`, `org import` | Full export and restore |
| `search reindex` | Rebuild the search index |
| `demo seed`, `demo remove`, `demo reset` | Demonstration data |
| `openapi` | Print the OpenAPI document |
| `catalog` | List what the build offers to plugins |
| `entity-schema` | Generate the SQL of a managed entity |
| `plugin new`, `plugin build`, `plugin pack` | Create, build and package a runtime plugin |
| `probe` | Check platform invariants without a database |

## serve

Starts the HTTP server on `HTTP_HOST:HTTP_PORT`, serving the API and the embedded web app. Applies migrations first when `RUN_MIGRATIONS` is true.

| Flag | Meaning |
| --- | --- |
| `--with-worker` | Also run the job worker and scheduler in the same process. Ignored, with a warning, when `DATABASE_URL` is not set |

```bash
aphel serve
aphel serve --with-worker
```

## worker

Runs the background job worker, the scheduler and the worker heartbeat. It needs `DATABASE_URL` and applies migrations first when `RUN_MIGRATIONS` is true. It stops gracefully on a termination signal and waits for running jobs. No flags. See [Operations](/_aphel/docs/admin/operations/).

```bash
aphel worker
```

## migrate

Applies pending core and module migrations, synchronizes system and custom-object fields, and exits. It runs regardless of `RUN_MIGRATIONS`. For production upgrades prefer `upgrade`.

With `CONTROL_DATABASE_URL` it migrates the control database and then every active or suspended tenant, one at a time; a tenant that fails keeps its error (see `tenants list`) and the others go on. `--tenant <slug>` migrates only that tenant.

```bash
aphel migrate
aphel migrate --tenant acme
```

## tenants

Manages the tenants of a multi-tenant installation (`CONTROL_DATABASE_URL`). Every tenant database lives on the control server with the name `<control database>_<slug>`; the control credentials need `CREATE` and `DROP` privileges. Stored files of a tenant live under `<slug>/` in the configured storage. Each action is recorded in the control database (`operator_audit`).

| Command | Meaning |
| --- | --- |
| `tenants list` | Slug, state, name, database, hosts, last migration and last error of each tenant |
| `tenants create --slug <slug> --name <name> --host <host> --admin-email <email> --admin-name <name>` | Registers the tenant as provisioning, creates and migrates its database, installs the organization with its first administrator and activates it. `--host` repeats (the first is the primary); `--blueprint`, `--module` (repeats), `--timezone`, `--currency`, `--locale` and `--public-url` (default `https://` plus the primary host) choose the installation. The administrator password is read without echo, or from standard input when it is not a terminal. A failed creation keeps the tenant as provisioning with its error; delete it with `--no-export` and create it again |
| `tenants suspend <slug>`, `tenants resume <slug>` | Stop or restart serving the tenant and its jobs; the data is kept |
| `tenants delete <slug> --confirm <slug> (--export <zip> \| --no-export)` | Marks the tenant as deleting, writes `org export` to the zip, removes its stored files, drops its database and its registration. Cannot be undone |
| `tenants migrate [<slug>]` | Same as `migrate` with or without `--tenant` |

Commands that act on one organization (`doctor`, `modules`, `mounts`, `secrets rotate`, `search`, `org`, `demo`) take `--tenant <slug>` in a multi-tenant installation and refuse to run without it. `db reset` and `upgrade` only work on a single-company installation.

The console on `CONSOLE_HOST` offers the same actions to operators, plus hosts, health and the action history. Its first operator comes from the command line; the others are invited from the console with a link valid for 7 days:

```bash
aphel console create-operator --email ops@example.com --name "Operations"
```

`aphel tenancy-check` creates two throwaway tenants, checks through the real host routing that neither reaches the other's database, files, cached state, modules, sessions, records or events, and deletes them, also on failure. It exits 1 when a check fails. See [Serving many companies](/_aphel/docs/admin/multi-tenancy/).

```bash
aphel tenants create --slug acme --name "Acme" --host acme.example.com \
  --admin-email ana@acme.example --admin-name "Ana Souza" --blueprint crm
aphel modules list --tenant acme
aphel tenants delete acme --confirm acme --export acme.zip
```

## upgrade

Checks the requirements, lists pending migrations, asks for a backup confirmation and applies the migrations. See [Upgrading](/_aphel/docs/admin/upgrading/).

| Flag | Meaning |
| --- | --- |
| `--check` | Only check the requirements and list pending migrations. Changes nothing |
| `--backup-confirmed` | Confirm that a backup of the database and files was taken right before. Required when there is no terminal |

```bash
aphel upgrade --check
aphel upgrade --backup-confirmed
```

## doctor

Prints a Markdown diagnostic report: version, operating system, MySQL version, the Aphel environment variables with secrets redacted, health checks (database, migrations, worker, scheduler, queue, storage, disk, SMTP, encryption key, metrics), the compiled modules and the worker heartbeats. Safe to attach to a bug report. No flags.

```bash
aphel doctor > aphel-doctor.md
```

## healthcheck

Exits 0 when healthy, and prints `healthy: ...` or `unhealthy: ...`. Meant for container and orchestrator probes.

| Flag | Meaning |
| --- | --- |
| (none) | Call `GET /_aphel/health` on the local server (`HTTP_HOST` is replaced by loopback if it is `0.0.0.0`) |
| `--ready` | Call `GET /_aphel/health/ready`: database, storage, worker, scheduler and queue. Cannot be combined with `--worker` |
| `--worker` | Check that the worker on this host sent a heartbeat within `HEALTH_WORKER_MAX_SILENCE_SECS`. Reads the database instead of calling HTTP |

```bash
aphel healthcheck
aphel healthcheck --ready
aphel healthcheck --worker
```

## secrets

| Subcommand | Meaning |
| --- | --- |
| `generate-key` | Print a new random value for `APP_ENCRYPTION_KEY` (32 random bytes, base64). Needs no database |
| `rotate` | Re-encrypt every stored secret, and the two-factor secrets, with `APP_ENCRYPTION_KEY`. The old keys must be in `APP_ENCRYPTION_PREVIOUS_KEYS` |

```bash
aphel secrets generate-key
aphel secrets rotate
```

See [Security](/_aphel/docs/admin/security/#rotate-the-encryption-key) for the full rotation procedure.

## db reset

Permanently deletes the database named in `DATABASE_URL`, recreates it and applies all migrations. This removes the company, users, records and database secrets and makes the installer available again. Stored files are not deleted. Stop every server and worker first. The database user needs `DROP` and `CREATE`.

| Flag | Meaning |
| --- | --- |
| `--confirm DATABASE` | Required. Must equal the exact database name in `DATABASE_URL` |

The command refuses names that are longer than 64 characters or use characters other than ASCII letters, digits, underscore, hyphen and dollar sign, and it refuses the system databases `mysql`, `information_schema`, `performance_schema` and `sys`.

```bash
aphel db reset --confirm aphel
```

## modules

| Subcommand | Meaning |
| --- | --- |
| `list` | Table of the modules compiled into the build, in load order: key, version, required, enabled, compatibility and dependencies. "Enabled" is read from the database and shows `?` when it cannot be read |
| `doctor` | What each module contributes (routes, hooks, services, catalogs, jobs, consumers, schedules) and the conflicts between modules |
| `uninstall KEY` | Disable a module and run its uninstall step. A required module cannot be uninstalled |

| Flag of `uninstall` | Meaning |
| --- | --- |
| `--purge` | Also export the module's data to `aphel-module-<key>-<timestamp>.json` in the current directory, remove the data (tables, settings, secrets) and revert the module's schema |

```bash
aphel modules list
aphel modules uninstall insights
aphel modules uninstall insights --purge
```

See [Modules](/_aphel/docs/admin/modules/).

## mounts reset

Moves the internal app back to `/` when a wrong app path (Settings → General) left it unreachable through the proxy. The change goes through the same organization settings update as the screen, so it publishes `organization.settings_updated` and writes the audit entry. Running servers pick it up without a restart. Before resorting to it, try `/_aphel/app/`, where the app always answers.

```bash
aphel mounts reset
```

## org

Added by the `org_export` module. Needs `DATABASE_URL`.

### org export

Writes a full backup of the installation (records, relationships, files, configuration, members and a manifest) to a zip file.

| Flag | Meaning |
| --- | --- |
| `--output PATH` | Required. Path of the zip to write |
| `--without-files` | Leave attached files out |
| `--as EMAIL` | Email of the active user whose access is used. Defaults to the first administrator |

### org import

Restores a full backup into an empty installation that is not set up yet. Takes the archive path as its argument. It rebuilds the search index afterwards.

```bash
aphel org export --output /srv/backups/aphel-full.zip
aphel org export --output /srv/backups/aphel-light.zip --without-files --as admin@example.com
aphel org import /srv/backups/aphel-full.zip
```

See [Backup and restore](/_aphel/docs/admin/backup-and-restore/).

## search reindex

Added by the `search` module. Rebuilds the global search index.

| Flag | Meaning |
| --- | --- |
| `--record-type TYPE` | Reindex one record type, such as `contact` or `deal`. Default: all types |

```bash
aphel search reindex
aphel search reindex --record-type deal
```

## demo

Added by the `demo` module. Creates fictional data for demonstrations and tests. Applies migrations first when `RUN_MIGRATIONS` is true.

| Subcommand | Meaning |
| --- | --- |
| `seed` | Fill the installation with fictional data. Refuses when it already has records unless `--force` |
| `remove` | Delete the fictional data created by `seed` |
| `reset` | Delete the demo data and seed again |

| Flag | Applies to | Meaning |
| --- | --- | --- |
| `--force` | `seed` | Seed even when the installation already has records |
| `--full` | `reset` | Also delete every record, file and non-administrator user, including what visitors created, before seeding |
| `--visitor-email EMAIL` | `seed`, `reset` | Email of the limited demo visitor to create or reset. Defaults to `DEMO_USER_EMAIL` when `DEMO_MODE` is on |
| `--visitor-password PASSWORD` | `seed`, `reset` | Password of that visitor. Defaults to `DEMO_USER_PASSWORD` |

```bash
aphel demo seed
aphel demo reset --full
```

Do not seed demo data into a production installation. The `docker/demo` folder in the repository holds a compose file and a daily reset script for a public demo.

## openapi

Prints the OpenAPI document of the API as JSON to standard output. No database needed.

`--console` prints the API of the multi-tenant console instead (`back/console-openapi.json`, from which `make api-types` generates `web/src/console/api-types.d.ts`).

```bash
aphel openapi > openapi.json
aphel openapi --console > console-openapi.json
```

A running server also serves it at `/api/openapi.json`, with interactive docs at `/api/docs`. See the [REST API reference](/_aphel/docs/reference/rest-api/).

## catalog

Lists everything the build offers to plugins: plugins, operations, hooks, events and permissions. No database needed. See [Plugins overview](/_aphel/docs/plugins/overview/).

| Flag | Meaning |
| --- | --- |
| `--json` | Print the whole catalog as JSON |
| `--markdown hooks` or `--markdown host` | Print a generated reference document |
| `--sdk rust`, `--sdk js` or `--sdk js-types` | Print the generated catalog file for the Rust or JavaScript plugin SDK |

Without flags it prints a summary.

```bash
aphel catalog
aphel catalog --json
```

## entity-schema

Generates the DDL of a managed entity from a JSON declaration, without connecting to a database. It prints SQL statements.

| Argument | Meaning |
| --- | --- |
| `--key KEY` | Required. Entity key, used for its permission `<key>.read` |
| `DEFINITION` | Path to the JSON declaration file |

```bash
aphel entity-schema --key invoice invoice.json
```

See [Entities and fields](/_aphel/docs/plugins/entities-and-fields/).

## probe

Checks the platform invariants of this build (access filters, hook order, registry, operations and event actors). It does not load the configuration or connect to a database, and it prints failures only. Mostly useful when developing modules.

```bash
aphel probe
```

## plugin

Creates, builds and packages runtime plugins. It does not load the configuration or connect to a database. The same commands exist as the standalone `aphel-plugin` binary, which is small and can be installed without the server.

| Command | What it does |
| --- | --- |
| `aphel plugin new <key> [--js] [--sdk <path>]` | Creates the directory `aphel-<key>` with a manifest and a starter backend (Rust by default, JavaScript with `--js`). `--sdk` points to the plugin SDK of your checkout |
| `aphel plugin build [dir]` | Builds the backend and the web bundle of the plugin in `dir` (the current directory by default) |
| `aphel plugin pack [dir] [-o file.aphelplugin]` | Builds, then writes `<key>-<version>.aphelplugin` (or the file given with `-o`) |

```bash
aphel plugin new library --js
aphel plugin pack aphel-library -o library.aphelplugin
```

See [Packaging](/_aphel/docs/plugins/packaging/) and [Your first plugin](/_aphel/docs/plugins/first-plugin/).
