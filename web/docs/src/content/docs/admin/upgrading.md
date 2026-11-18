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

1. **Back up** the database and the uploaded files, right before the upgrade. See [Backup and restore](/_aphel/docs/admin/backup-and-restore/).
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

6. **Start** the new `aphel serve` and `aphel worker`, then check `/_aphel/health/ready` or the System screen.

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

On success the command prints how many migrations were applied and from which version to which. It synchronizes system fields and custom-object fields, and writes a record of the upgrade (versions, applied migrations, host, outcome) in the `system_upgrades` table. It then asks you to restart `aphel serve` and `aphel worker` and to check `/_aphel/health/ready`.

The System screen shows the migration state and warns when migrations are pending. Modules whose version is newer than the version installed for the organization show "Update pending" on the Modules screen. See [Modules](/_aphel/docs/admin/modules/).

## When an upgrade fails

MySQL commits schema changes one by one, so a failed upgrade can leave the database partially migrated. The command records the failure and prints the migrations that were applied before it stopped. To recover:

1. Stop every Aphel process.
2. Restore the backup taken before the upgrade, with `restore.sh`. See [Backup and restore](/_aphel/docs/admin/backup-and-restore/).
3. Put the previous binary or image back and start it.
4. Run `aphel doctor` and attach its report if you open an issue.

## Downgrades

A binary older than the database refuses to start on migrations it does not know, and `aphel upgrade` reports them as an inconsistent history. To go back, restore a backup made with the older version.

## Version compatibility of backups

Restore a backup with the same Aphel version that produced it, or a newer one. Pending migrations then run at start or through `aphel upgrade`.

## Update from the interface

In a single-company installation, an administrator with `system.manage` finds **System updates** on the **System** screen. In a multi-company installation, the **Updates** page is in the operator console. Company administrators cannot update the shared installation, even by calling the company API directly. Requests require browser sessions; API tokens cannot start system updates.

The screen shows the installed version, the latest stable release, release notes, progress and history. **Check for updates** reads the latest non-draft, non-prerelease GitHub release. The external service also checks every six hours. Builds of `master` (`edge`) are not stable releases and do not appear as available updates. Publish a version tag matching `back/Cargo.toml` to make a stable release available after its release workflow succeeds.

**Prepare update** downloads the official image, verifies its source and version, and pins its digest without interrupting the application. **Update now** requires a separate confirmation. In multi-company mode, the confirmation explains that every company will be unavailable during the update.

### Enable the external update service

This first implementation supports Linux hosts running the production Docker Compose stack with a dedicated MySQL service and a Docker volume mounted at `/app/storage`. Binary and externally managed database installations continue to use the manual upgrade procedure. No Docker socket is mounted inside the web application.

Install Python 3, Docker and the Compose plugin on the host. Copy `docker/updater/compose.updater.yml` into the installation directory. Add it after the installation's existing Compose files when installing the host service:

```bash
sudo docker/updater/install.sh /opt/aphel docker-compose.prod.yml compose.multi.yml compose.updater.yml
sudo aphel-compose up -d --no-build --wait
```

Omit `compose.multi.yml` for a single-company installation. The application image must include the update API. The installer creates `/etc/aphel/updater.json` (root-only), installs `aphel-updater.service` and exposes only `/run/aphel-updater/updater.sock`, writable by root and container group 10001. The application sees this socket through a read-only directory mount and communicates through a fixed API; it cannot submit shell commands, image names or backup paths.

If the container package is private, authenticate the host using a read-only registry credential:

```bash
sudo docker --config /etc/aphel/registry login ghcr.io
```

The credential stays on the host and is not sent to the application. Use `sudo aphel-compose` for subsequent Compose operations: it includes the installed configuration and any image digest selected by the updater. Calling only the original Compose file could select the old image again.

### Backup and recovery

Before installation, the updater checks disk space, stops the application and worker, and creates a backup in `/var/lib/aphel-updater/backups/<operation-id>/`. The backup contains all application databases in this stack's MySQL service (including the control database and every company database), local uploaded files, the installation environment and Compose files, and a SHA-256 manifest. Remote S3 objects are not copied. These backups contain secrets and remain readable only by root; keep an off-server copy under your backup policy.

A download or backup failure keeps the existing image. Once the new application has been started, its migrations may have changed schemas. If startup, health or isolation verification fails, the updater stops the application and worker and records **Recovery required**. It does not switch back to an old binary against an unknown database schema. An interrupted updater operation is also marked for recovery on restart. New updates are blocked until an administrator has recovered the installation.

Recovery is a server operation: stop `aphel-updater.service`, verify the backup manifest, restore `database.sql.gz` into this stack's MySQL service, restore `storage.tar.gz` into the application volume, and select the previous image override recorded in `previous-image.json`. Restore the saved installation environment if needed. Restart and verify the application and worker before clearing the recovery state. To clear it after successful recovery, move `/var/lib/aphel-updater/state.json` aside and restart the update service; retain the saved state and backups as the incident record. Do not clear recovery state merely to retry a failed migration.

After installing a release, the service waits for both containers to become healthy and verifies the installed version. Single-company installations must pass the readiness check; multi-company installations must pass `aphel tenancy-check`. The operation history records the initiating administrator or operator and the final outcome.
