---
title: Upgrading
description: Upgrade an installation safely with aphel upgrade, including the backup confirmation, the disk and database checks, and what to do when it fails.
sidebar:
  order: 10
---

Upgrading means replacing the binary or image with a newer one and applying the database migrations that come with it. The guarded way to do it is `aphel upgrade`. It checks the environment, shows exactly what will change, asks you to confirm that a backup exists, applies the migrations and records the result.

## What starts the migrations

| Path | Behavior |
| --- | --- |
| `aphel upgrade` | Checks requirements, lists pending migrations, requires a backup confirmation, then applies them. Recommended. |
| `aphel migrate` | Applies pending migrations and exits, with no checks and no confirmation. |
| `aphel serve` or `aphel worker` | Apply pending migrations at start when `RUN_MIGRATIONS` is true, which is the default, with no confirmation. |

To make sure nobody upgrades by simply starting a new binary, set `RUN_MIGRATIONS=false` in the environment and use `aphel upgrade` as the only path. Migrations are not reversible: MySQL does not roll back schema changes, so the backup is your way back.

## Upgrade steps

1. **Back up** the database and the uploaded files, right before the upgrade. See [Backup and restore](/docs/admin/backup-and-restore/).
2. **Put the new binary or image in place**, keeping the old one for rollback. The running server can stay up for now.
3. **Check.** Run the upgrade in check mode with the new binary. It reports the requirements and the pending migrations, and changes nothing.

   ```bash
   aphel upgrade --check
   ```

4. **Stop** `aphel serve` and `aphel worker`.
5. **Apply.** Confirm that you have a backup, either interactively or with the flag.

   ```bash
   aphel upgrade --backup-confirmed
   ```

6. **Start** the new `aphel serve` and `aphel worker`, then check `/health/ready` or the System screen.

### With Docker Compose

Pin the version with `APHEL_IMAGE` in `.env`, then:

```bash
docs/scripts/backup.sh --compose docker-compose.prod.yml --output /srv/backups/aphel
docker compose -f docker-compose.prod.yml pull app
docker compose -f docker-compose.prod.yml run --rm --no-deps app upgrade --check
docker compose -f docker-compose.prod.yml stop app worker
docker compose -f docker-compose.prod.yml run --rm --no-deps app upgrade --backup-confirmed
docker compose -f docker-compose.prod.yml up -d
```

The `app` image runs the `aphel` binary as its entrypoint, so the arguments after the service name are the subcommand.

## What `aphel upgrade` checks

The report lists each check with its status.

| Check | Fails when |
| --- | --- |
| database | The database in `DATABASE_URL` cannot be reached |
| mysql version | The server is older than MySQL 8.0, or it is MariaDB |
| file storage | A probe file cannot be written and deleted in the configured storage |
| disk space | Free space on the file storage folder or on the working directory is below `UPGRADE_MIN_FREE_DISK_MB` (default 500). Lower the variable or free space |
| migrations | The history is inconsistent: a core migration is marked as failed, the database holds a migration this binary does not know (the database is newer than the binary), or a module migration was changed after it was applied |

Then it prints the **pending migrations** (core and per module) and the **compiled modules** with their version, minimum Aphel version and compatibility. A module that is incompatible with this build has its pending migrations skipped, and it stays unavailable.

Any failed requirement aborts with "Nothing was changed". If no migration is pending, the command ends with "The database is already up to date" and asks for nothing.

## The backup confirmation

Before touching the schema, the command needs proof that you took a backup:

- On a terminal, it asks you to type `yes`, `y`, `sim` or `s`.
- Without a terminal (scripts, containers started without `-it`), it aborts unless you pass `--backup-confirmed`.

The flag is a statement of fact. Aphel does not verify the backup.

## After the upgrade

On success the command prints how many migrations were applied and from which version to which. It synchronizes system fields and custom-object fields, and writes a record of the upgrade (versions, applied migrations, host, outcome) in the `system_upgrades` table. It then asks you to restart `aphel serve` and `aphel worker` and to check `/health/ready`.

The System screen shows the migration state and warns when migrations are pending. Modules whose version is newer than the version installed for the organization show "Update pending" on the Modules screen. See [Modules](/docs/admin/modules/).

## When an upgrade fails

MySQL commits schema changes one by one, so a failed upgrade can leave the database partially migrated. The command records the failure and prints the migrations that were applied before it stopped. To recover:

1. Stop every Aphel process.
2. Restore the backup taken before the upgrade, with `restore.sh`. See [Backup and restore](/docs/admin/backup-and-restore/).
3. Put the previous binary or image back and start it.
4. Run `aphel doctor` and attach its report if you open an issue.

## Downgrades

A binary older than the database refuses to start on migrations it does not know, and `aphel upgrade` reports them as an inconsistent history. To go back, restore a backup made with the older version.

## Version compatibility of backups

Restore a backup with the same Aphel version that produced it, or a newer one. Pending migrations then run at start or through `aphel upgrade`.
