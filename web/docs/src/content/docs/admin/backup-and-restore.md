---
title: Backup and restore
description: What a complete Aphel backup contains, the backup.sh and restore.sh scripts, backing up S3 files, and the portable full export.
sidebar:
  order: 11
---

A complete backup has two parts that must be copied together, plus a copy of your environment file.

| Part | Where it lives | How to copy it |
| --- | --- | --- |
| MySQL database | The `mysql-data` volume (Docker) or your MySQL server | `mysqldump --single-transaction`, without stopping Aphel |
| Uploaded files (attachments, logos, exports) | The local storage folder (default `./storage`; Docker: the `app-storage` volume at `/app/storage`) or the S3 bucket, according to Settings, File storage | A `tar` of the folder, or a bucket sync with versioning |
| Environment file | `.env` or `/etc/aphel/aphel.env` | Copy it to a safe, separate place |

The database holds references to the files, so one without the other produces broken attachments. Settings made on screen (public address, SMTP, storage) live in the database, and their secrets are in the vault, encrypted with `APP_ENCRYPTION_KEY`. The environment file is **not** included in the scripts' backup. It holds passwords and the encryption key, and without the key restored secrets cannot be read.

## The backup and restore scripts

The repository includes two scripts in `docs/scripts/`.

| Script | What it does |
| --- | --- |
| `backup.sh` | Creates a folder `aphel-backup-<UTC date>` (for example `aphel-backup-20261020T190609Z`) with `database.sql.gz`, `storage.tar.gz` and `manifest.txt` (SHA-256 checksums). It checks that the dump finished by looking for the final `Dump completed` line. |
| `restore.sh` | Verifies the checksums in `manifest.txt`, **drops and recreates** the database, imports the dump and replaces the contents of the files folder. It asks for confirmation unless you pass `--yes`. |

The dump uses `--single-transaction`, `--routines`, `--triggers`, `--no-tablespaces` (so the Aphel user does not need the `PROCESS` privilege) and `--default-character-set=utf8mb4`. Restore recreates the database with `utf8mb4` and the collation `utf8mb4_0900_ai_ci`.

Both scripts work in two modes:

- **Docker** (`--compose FILE`): the dump runs inside the `mysql` service using its own `MYSQL_USER`, `MYSQL_PASSWORD` and `MYSQL_DATABASE`, so the host needs no MySQL client and no password on the command line. Files are read from and written to the `/app/storage` volume by a temporary container of the `app` service.
- **Manual** (no `--compose`): uses the host's `mysqldump` and `mysql`, with the connection in `DATABASE_URL` (`mysql://user:password@host:port/database`), and a files folder given by `--storage-dir`.

### backup.sh options

| Option | Meaning |
| --- | --- |
| `--compose FILE` | Docker mode, with the given Compose file |
| `--output DIR` | Folder where the backup folder is created. Default `./backups` |
| `--storage-dir DIR` | Local files folder in manual mode. Default `./storage` |
| `--skip-storage` | Do not copy files, for installations that keep files on S3 |
| `-h`, `--help` | Show help |

### restore.sh options

The argument is the backup folder.

| Option | Meaning |
| --- | --- |
| `--compose FILE` | Docker mode. It stops `app` and `worker`, starts `mysql`, restores, and leaves both stopped |
| `--storage-dir DIR` | Local files folder in manual mode. Default `./storage` |
| `--skip-storage` | Do not restore files |
| `--yes` | Do not ask for confirmation |
| `-h`, `--help` | Show help |

Without `--yes`, `restore.sh` asks you to type `sim` ("yes" in Portuguese). Any other answer cancels without touching anything. The messages of both scripts are in Portuguese.

## Docker

In the repository folder (where `.env` is):

```bash
docs/scripts/backup.sh --compose docker-compose.prod.yml --output /srv/backups/aphel
```

Restore, then start the stack again:

```bash
docs/scripts/restore.sh --compose docker-compose.prod.yml /srv/backups/aphel/aphel-backup-20261020T190609Z
docker compose -f docker-compose.prod.yml up -d
```

## Manual installation

With the scripts copied to `/opt/aphel/`, and the layout from [Installation](/_aphel/docs/admin/installation/):

```bash
sudo -u aphel sh -c 'set -a; . /etc/aphel/aphel.env; exec /opt/aphel/backup.sh --storage-dir /var/lib/aphel/storage --output /srv/backups/aphel'
```

To restore, stop the services first:

```bash
sudo systemctl stop aphel aphel-worker
sudo -u aphel sh -c 'set -a; . /etc/aphel/aphel.env; exec /opt/aphel/restore.sh --storage-dir /var/lib/aphel/storage /srv/backups/aphel/aphel-backup-20261020T190609Z'
sudo systemctl start aphel aphel-worker
```

The database user must be able to drop and create the Aphel database. `GRANT ALL PRIVILEGES ON aphel.*` is enough.

## Schedule and retention

A daily backup at 3 a.m. for Docker, keeping 14 days:

```text
0 3 * * * cd /srv/aphel && docs/scripts/backup.sh --compose docker-compose.prod.yml --output /srv/backups/aphel >> /var/log/aphel-backup.log 2>&1 && find /srv/backups/aphel -maxdepth 1 -name 'aphel-backup-*' -mtime +14 -exec rm -rf {} +
```

- Copy backups off the server (another provider, a versioned bucket, `rclone`, `restic`). A backup on the same disk does not protect against losing the server.
- Backups contain personal data and password hashes. Store them encrypted and with restricted access.
- **Test the restore regularly** in a separate environment.

## Files on S3

With S3 storage, files live in the configured bucket with keys of the form `<year>/<month>/<file id>`. Options:

- Enable bucket **versioning** and a lifecycle rule for old versions.
- Sync the bucket to another destination at the same time as the dump, for example `aws s3 sync s3://aphel /srv/backups/aphel-s3`, or `rclone sync source:aphel destination:aphel-backup`.

Run the scripts with `--skip-storage`. To restore, put the objects back into the bucket configured in Aphel with the same keys, and run `restore.sh --skip-storage` for the database.

## Without the scripts

```bash
mysqldump -h HOST -u aphel -p --single-transaction --routines --triggers --no-tablespaces aphel | gzip > database.sql.gz
tar -C /var/lib/aphel/storage -czf storage.tar.gz .
```

```bash
mysql -h HOST -u aphel -p -e 'DROP DATABASE IF EXISTS aphel; CREATE DATABASE aphel CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;'
gunzip -c database.sql.gz | mysql -h HOST -u aphel -p aphel
tar -C /var/lib/aphel/storage -xzf storage.tar.gz
```

## After a restore

- Restore with the same Aphel version that made the backup, or a newer one. Pending migrations then run at start, or through `aphel upgrade`. An older version refuses to start on a database with migrations it does not know.
- Sessions that were open at backup time become valid again. After a security incident, ask users to sign out of all devices or change passwords.
- Put the same `APP_ENCRYPTION_KEY` back, or secrets stored in the database cannot be decrypted.

## Full export: a portable backup

The **full export** is independent of the database schema. It writes a zip with a manifest (format `aphel.organization_export`, the Aphel version, the source organization, counts and a SHA-256 of every entry), the configuration package (empty when the Configuration packages module is off; pipelines, stages, lost reasons, tags and activity types then travel as plain rows), one JSON Lines file per record type (including custom objects), relationships, notes, history, members and the attached files.

| Task | How |
| --- | --- |
| Export from the app | Settings, Full export. A worker builds the zip. Only one export can run at a time. Needs the `organization.export` permission |
| Export from the command line | `aphel org export --output FILE` |
| Restore | In the installer (see [Installation](/_aphel/docs/admin/installation/)) or with `aphel org import FILE`, on an installation that is not set up yet |

```bash
aphel org export --output /srv/backups/aphel-full.zip
aphel org import /srv/backups/aphel-full.zip
```

`aphel org export` accepts `--without-files` to leave attachments out, and `--as EMAIL` to choose the active user whose access is used (by default the first administrator). Importing restores into an empty, not-yet-installed installation and rebuilds the search index afterwards. The installer upload is limited to 2 GiB, so use `aphel org import` for larger archives.

Use the full export to move data between installations. Use the scripts for routine backups of a running installation.
