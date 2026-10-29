# Upgrading

Aphel versions are published as `vX.Y.Z` tags in the repository, with the changes described in `CHANGELOG.md`. Read the release notes before upgrading: changes that require manual action are listed there.

## How migrations work

- Database migrations are embedded in the binary (`back/kernel/migrations/`) and applied in order. Each one runs once; MySQL keeps the history in the `_sqlx_migrations` table.
- By default, `aphel serve` and `aphel worker` apply pending migrations **at startup**, and then sync the organization's default data (system fields, pipeline, activity types and loss reasons). Both can start at the same time: the application uses a database lock.
- `aphel migrate` does the same and exits, without starting the server.
- Migrations **cannot be rolled back**. An older Aphel version refuses to start on a database that has already received migrations from a newer version. Going back a version = restoring the backup taken before the upgrade.
- In MySQL, schema changes (DDL) are not transactional: if a migration fails midway, the database may be left partially changed. That is another reason the backup beforehand is mandatory.

## Step by step with Docker

```bash
cd /path/to/aphel
docs/scripts/backup.sh --compose docker-compose.prod.yml --output /srv/backups/aphel
git fetch --tags
git checkout v0.2.0
sed -i 's|^APHEL_IMAGE=.*|APHEL_IMAGE=ghcr.io/viniciusadm/aphel:0.2.0|' .env
docker compose -f docker-compose.prod.yml pull app
docker compose -f docker-compose.prod.yml up -d
docker compose -f docker-compose.prod.yml logs -f app worker
```

1. **Backup** (see [Backup and restore](backup.md)). Check that the folder was created with `database.sql.gz` and `storage.tar.gz`.
2. Update the repository to the desired version (replace `v0.2.0` with the tag): the compose file and the scripts follow the version.
3. Point `APHEL_IMAGE` in `.env` to the same version (if the line does not exist, add it). Without `APHEL_IMAGE`, `latest` is used, and `pull` fetches the latest stable version. Also compare the new `.env.example` with your `.env` and add new variables, if any.
4. `pull` downloads the new image and `up -d` recreates `app` and `worker`. `app` applies the migrations at startup. If you build the image locally, replace both commands with `up -d --build`.
5. In the logs, look for `running database migrations` followed by `backend listening on ...`. Then check `/health` and log in.

## Step by step on a manual installation

```bash
sudo -u aphel sh -c 'set -a; . /etc/aphel/aphel.env; exec /opt/aphel/backup.sh --output /srv/backups/aphel'
sudo systemctl stop aphel aphel-worker
sudo install -m 0755 aphel-new /usr/local/bin/aphel
sudo -u aphel sh -c 'set -a; . /etc/aphel/aphel.env; exec aphel migrate'
sudo systemctl start aphel aphel-worker
journalctl -u aphel -u aphel-worker -f
```

Stopping both services first prevents an old process from working with the already migrated database. The explicit `migrate` is optional when `RUN_MIGRATIONS` is on, but it shows any error before starting the server.

## Guided upgrade with `aphel upgrade`

`aphel upgrade` does the same as `migrate`, but with checks before and a record afterwards. Use it with the **new** binary (or image), with `serve` and `worker` stopped and the same variables as the service:

```bash
aphel upgrade --check               # only checks and lists what will run; changes nothing
aphel upgrade                       # asks whether there is a recent backup (type "yes")
aphel upgrade --backup-confirmed    # for scripts: confirms the backup without asking
# Docker
docker compose -f docker-compose.prod.yml run --rm app upgrade --backup-confirmed
```

What it does, in order:

1. **Requirements**: connects to the database, checks the MySQL version (8.0 or newer; MariaDB is not supported) and writes and deletes a test file in the configured storage (disk or S3). It also checks the free disk space of the local storage directory (when Settings → File storage uses the disk) and of the current directory, and aborts if either has less than `UPGRADE_MIN_FREE_DISK_MB` (default `500`; `0` disables the check). If the space cannot be read (non-Unix system), the check shows as `skipped` and does not block.
2. **Migration history**: refuses to proceed if a core migration was marked as failed, if the database has migrations this binary does not know (database newer than the binary) or if a module's migration changed after being applied.
3. **Plan**: lists the pending migrations of the core and of the compiled modules and the compatibility of each module (minimum Aphel version and dependencies). Incompatible modules do not have their migrations applied and stay unavailable until they are updated.
4. **Backup**: without `--backup-confirmed` and outside an interactive terminal, or without the answer `yes`, it **aborts without changing anything**. Take the backup with `docs/scripts/backup.sh` (see [Backup and restore](backup.md)).
5. **Execution**: applies the core migrations, then the modules' ones, and syncs the organization's default data (like `migrate`).
6. **Record**: stores the run in the `system_upgrades` table (from/to version, last migration before and after, applied migrations, host, time and error, if any). The records stay in the database for reference.

If nothing is pending, it says so and exits without asking for a backup. If a migration fails, it shows the error, which migrations were applied and the recovery instructions:

1. stop all Aphel processes (`serve` and `worker`);
2. restore the backup taken beforehand (`docs/scripts/restore.sh <backup-folder>`, with `--compose docker-compose.prod.yml` on Docker);
3. go back to the previous binary or image and start it again;
4. run `aphel doctor` and attach the report when opening an issue with the error.

The exit code is non-zero when it aborts or fails, so it can be used in deployment scripts. The error message appears only once (`Upgrade aborted: ...` or `Upgrade FAILED: ...`), on standard error.

## Turning off automatic migrations

In environments where schema changes must be controlled (maintenance window, approval, multiple app replicas), set:

```bash
RUN_MIGRATIONS=false
```

With that, `serve` and `worker` do not apply migrations or sync the default data at boot. On every upgrade, run `migrate` before starting the new version:

```bash
docker compose -f docker-compose.prod.yml pull app
docker compose -f docker-compose.prod.yml stop app worker
docker compose -f docker-compose.prod.yml run --rm app migrate
docker compose -f docker-compose.prod.yml up -d
```

On a manual installation, run `aphel migrate` with the same variables as the service. Forgetting `migrate` leaves the new code running on the old schema, with errors on the routes that depend on the new tables.

## Going back to the previous version

1. Stop `app` and `worker`.
2. Revert the code (or the binary) to the previous version.
3. Restore the backup taken before the upgrade with `restore.sh` (database and files).
4. Start it again.

Data created between the upgrade and the rollback is lost; that is why you should check the new version right after upgrading.
