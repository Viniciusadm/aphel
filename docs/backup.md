# Backup and restore

A complete Aphel backup has two parts, which must be copied together:

| Part | Where it lives | How to copy |
| --- | --- | --- |
| MySQL database | `mysql-data` volume (Docker) or your MySQL server | `mysqldump --single-transaction` (without stopping Aphel) |
| Uploaded files (attachments, logos, exports) | Local storage directory (default `./storage`; Docker: `app-storage` volume, at `/app/storage`) or the S3 bucket, according to Settings → File storage | `tar` of the directory, or bucket sync/versioning |

The database holds the references to the files; one without the other produces broken attachments. Settings made on screen (public address, SMTP, storage) live in the database, and their secrets in the vault encrypted with `APP_ENCRYPTION_KEY`. The `.env` (or `/etc/aphel/aphel.env`) is not included in the automatic backup: keep a copy of it in a safe, separate place, because it contains passwords and the encryption key (without it the restored secrets are not readable).

To move the data to another installation independently of the database schema, use the [full export](data/org-export-format.md) (`aphel org export`), which can be restored into an empty installation through the installer or with `aphel org import`.

## Example scripts

The repository includes two scripts in [`docs/scripts/`](scripts/), tested as described in [Tested restore](#tested-restore):

| Script | What it does |
| --- | --- |
| [`backup.sh`](scripts/backup.sh) | Creates `aphel-backup-<UTC date>/` with `database.sql.gz`, `storage.tar.gz` and `manifest.txt` (SHA-256 checksums). Checks that the dump finished (`Dump completed`) |
| [`restore.sh`](scripts/restore.sh) | Verifies the checksums, **drops and recreates** the database, imports the dump and replaces the contents of the files directory. Asks for confirmation (type `sim`) unless it receives `--yes` |

Both work in two modes:

- `--compose docker-compose.prod.yml`: Docker installation. The dump runs inside the `mysql` service with its credentials (`MYSQL_USER`, `MYSQL_PASSWORD`, `MYSQL_DATABASE`), so the host needs neither a MySQL client nor a password on the command line. Files are read from and written to the volume by a temporary container of the `app` service.
- Without `--compose`: manual installation. Uses the host's `mysqldump`/`mysql`, with the `DATABASE_URL` connection, and the `--storage-dir` directory (default `./storage`).

With files on S3, use `--skip-storage` and back up the bucket separately (see [Files on S3](#files-on-s3)).

`mysqldump` options used: `--single-transaction` (consistent copy without locking InnoDB tables), `--routines`, `--triggers`, `--no-tablespaces` (avoids the `PROCESS` privilege, which the Aphel user does not have) and `--default-character-set=utf8mb4`.

### Docker

In the repository folder (where the `.env` is):

```bash
docs/scripts/backup.sh --compose docker-compose.prod.yml --output /srv/backups/aphel
```

Restore (stops `app` and `worker`, restores and leaves both stopped for you to check):

```bash
docs/scripts/restore.sh --compose docker-compose.prod.yml /srv/backups/aphel/aphel-backup-20261020T190609Z
docker compose -f docker-compose.prod.yml up -d
```

### Manual installation

```bash
sudo -u aphel sh -c 'set -a; . /etc/aphel/aphel.env; exec /opt/aphel/backup.sh --storage-dir /var/lib/aphel/storage --output /srv/backups/aphel'
```

(with the scripts copied to `/opt/aphel/`). To restore, stop the services first:

```bash
sudo systemctl stop aphel aphel-worker
sudo -u aphel sh -c 'set -a; . /etc/aphel/aphel.env; exec /opt/aphel/restore.sh --storage-dir /var/lib/aphel/storage /srv/backups/aphel/aphel-backup-20261020T190609Z'
sudo systemctl start aphel aphel-worker
```

The database user must be able to drop and create the Aphel database (`GRANT ALL PRIVILEGES ON aphel.*` is enough).

### Scheduling and retention

Example of a daily cron at 3 a.m. (Docker), keeping 14 days:

```cron
0 3 * * * cd /srv/aphel && docs/scripts/backup.sh --compose docker-compose.prod.yml --output /srv/backups/aphel >> /var/log/aphel-backup.log 2>&1 && find /srv/backups/aphel -maxdepth 1 -name 'aphel-backup-*' -mtime +14 -exec rm -rf {} +
```

Recommendations:

- Copy the backups off the server (another provider, a bucket with versioning, `rclone`, `restic`). A backup only on the same disk does not protect against losing the server.
- Backups contain personal data and password hashes: store them encrypted and with restricted access.
- **Test the restore periodically** in a separate environment, following the walkthrough below.

## Files on S3

With S3 storage (Settings → File storage), the files live in the configured bucket, with keys `<year>/<month>/<file id>`. Options:

- Enable bucket **versioning** and a lifecycle rule for old versions (protects against accidental deletion).
- Sync the bucket to another destination at the same time as the dump, for example `aws s3 sync s3://aphel /srv/backups/aphel-s3` or `rclone sync source:aphel destination:aphel-backup`.

When restoring, put the objects back into the bucket configured in Aphel with the same keys and run `restore.sh --skip-storage` for the database.

## Manual restore (without the scripts)

```bash
mysqldump -h HOST -u aphel -p --single-transaction --routines --triggers --no-tablespaces aphel | gzip > database.sql.gz
tar -C /var/lib/aphel/storage -czf storage.tar.gz .
```

```bash
mysql -h HOST -u aphel -p -e 'DROP DATABASE IF EXISTS aphel; CREATE DATABASE aphel CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;'
gunzip -c database.sql.gz | mysql -h HOST -u aphel -p aphel
tar -C /var/lib/aphel/storage -xzf storage.tar.gz
```

Restore with the same Aphel version that produced the backup (or a newer one: pending migrations run at boot). An older version refuses to start on a database with migrations it does not know.

After restoring, the sessions open at the time of the backup become valid again. If the restore is due to a security incident, ask users to sign out of all devices or change the passwords.

## Tested restore

Procedure performed on **2026-10-20**, in a clean, isolated environment (disposable containers and directories, unrelated to the development database), to validate the scripts. The commands below are already in the single-installation format (installer instead of sign-up, storage configured on screen and `--storage-dir` in the scripts).

Versions:

| Item | Version |
| --- | --- |
| Aphel | 0.1.0 (debug build, commit `c8b9707`) |
| MySQL (server) | 8.4.11 (image `mysql:8.4`) |
| `mysqldump` | 8.0.46 on the host (manual mode) and 8.4.11 in the container (`--compose` mode) |
| Docker Compose | v5.5.1 |
| System | Linux x86_64 (Ubuntu 24.04) |

### 1. Disposable source and destination

```bash
docker run -d --name aphel-backup-src -e MYSQL_ROOT_PASSWORD=root -e MYSQL_DATABASE=aphel \
  -e MYSQL_USER=aphel -e MYSQL_PASSWORD=aphel -p 3321:3306 mysql:8.4
docker run -d --name aphel-backup-dst -e MYSQL_ROOT_PASSWORD=root -e MYSQL_DATABASE=aphel \
  -e MYSQL_USER=aphel -e MYSQL_PASSWORD=aphel -p 3322:3306 mysql:8.4
```

The `aphel` user only has privileges on the `aphel` database (as in the production image).

### 2. Real data at the source

Aphel pointing to the source, running in a temporary directory (`src-run/`), with the default local storage at `src-run/storage`:

```bash
cd src-run && DATABASE_URL=mysql://aphel:aphel@127.0.0.1:3321/aphel HTTP_PORT=3301 aphel serve
```

Through the API (all non-GET calls with `X-Requested-With: aphel`):

| Step | Call | Result |
| --- | --- | --- |
| Installation (organization "Teste Backup" + admin `ana@backup.test`) | `POST /api/v1/installer` | 201 |
| Contact "Carlos Restaurado" | `POST /api/v1/people` | 201, mobile normalized to `+5511988887777` |
| Attachment `anexo.csv` on the contact | `POST /api/v1/files` (multipart with `record_type=contact` and `record_id`) | 201; file written to `src-run/storage/2026/10/<id>`, SHA-256 `bf82f810…8923e` |

### 3. Backup with Aphel running

```bash
DATABASE_URL=mysql://aphel:aphel@127.0.0.1:3321/aphel \
  docs/scripts/backup.sh --storage-dir $PWD/src-run/storage --output $PWD/backups
```

Output: `database.sql.gz` (15 KB, 40 tables, final line `Dump completed`), `storage.tar.gz` and `manifest.txt`. Exit code 0.

### 4. Destroying the source

Server stopped, `docker rm -f aphel-backup-src` and `rm -rf src-run`. Only the backup folder was left from the source.

### 5. Restore at the destination

```bash
DATABASE_URL=mysql://aphel:aphel@127.0.0.1:3322/aphel \
  docs/scripts/restore.sh --yes --storage-dir $PWD/dst-run/storage backups/aphel-backup-20261020T190609Z
```

SHA-256 checksums verified, database recreated, dump imported, file extracted to `dst-run/storage/`. Exit code 0. Without `--yes`, answering anything other than `sim` cancels without touching anything (verified).

### 6. Verification at the destination

Aphel pointing to the destination (`DATABASE_URL=...:3322/aphel`, running in `dst-run/`):

| Check | Result |
| --- | --- |
| Boot | No pending migrations (37 applied), no errors or warnings in the log |
| `POST /api/v1/auth/login` with `ana@backup.test` | 200 |
| `GET /api/v1/people/{id}` and search `?search=Restaurado` | 200, contact found (`total: 1`) |
| `GET /api/v1/attachments?record_type=contact&record_id={id}` | 200, one attachment |
| `GET /api/v1/files/{id}` (download) | 200, SHA-256 identical to the original file's |
| Session cookie issued by the source | Still valid at the destination (`GET /auth/me` 200) |

### 7. Docker mode (`--compose`)

The same scripts were exercised with `--compose` in two disposable Compose projects with the same structure as `docker-compose.prod.yml` (`mysql`, `app` and `worker` services, volume mounted at `/app/storage`; `app` used a lightweight image instead of the Aphel image, because the test only validates the backup commands):

1. `restore.sh --compose` in project A from the backup of step 3: stopped `app` and `worker`, recreated the database, imported the dump and restored the volume.
2. `backup.sh --compose` in project A: dump produced by the container's `mysqldump` 8.4.11, files read from the volume.
3. `restore.sh --compose` in project B from that backup: database with the contact and the user, and the file in the volume with the same SHA-256.

### 8. Cleanup

Server stopped, `docker rm -f aphel-backup-dst`, `docker compose down -v` in the test projects and removal of the temporary directories.

Note: the Aphel Docker image (`docker/Dockerfile`) was not built in this test; the `--compose` mode was validated with a stand-in container for the `app` service, which in real Aphel runs as the `aphel` user (UID 10001), owner of `/app/storage`.
