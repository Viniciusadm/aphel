# Full organization export

The full export generates a single `.zip` file with all of the installation's data (one installation = one company): records, links, history, attachments, configuration and members. It serves as a logical backup of the installation, for auditing, for leaving the service (portability) and for taking everything to another Aphel installation, restoring it into an empty installation (`aphel org import` or the installer's **Restore from a backup** step).

Unlike the database backup ([`backup.md`](../backup.md)), the export does not depend on the internal schema: the format below is stable and versioned.

## How to generate it

- **From the interface:** *Settings → Full export → Build export*. The package is built in the background by the worker (job `organization_export.run`); the requester receives the "Your export is ready" notification (or a failure one) and downloads the zip on the same screen. Only one export is queued at a time.
- **Through the API:** `POST /api/v1/organization-exports` (`{ "include_files": true }`), follow it with `GET /api/v1/organization-exports/{id}` and download it with `GET /api/v1/organization-exports/{id}/download`.
- **From the command line** (no worker, useful during maintenance): `aphel org export --output file.zip [--without-files] [--as email]` (`--as`: active user whose access is used; by default, the first administrator).

It requires the `organization.export` permission (the Administrator role already has it). The download only goes through the dedicated endpoint (`/organization-exports/{id}/download`): the generic download `GET /files/{id}` responds 404 for the zip, even for those with `settings.manage` (the same applies to privacy packages; files from list exports only come out there for whoever requested the export). The zip is kept in the file storage (`FileStorage`) like the other generated files.

The zip is written by streaming to a temporary file (the system's temporary directory, `TMPDIR`): each JSON Lines file is written page by page and each attachment is copied from the storage straight into the zip, without gathering everything in memory. Then the file is sent to the storage in parts (multipart upload on S3, a staging file on local disk) and the temporary file is deleted. Reserve free space in the temporary directory the size of the export. `aphel org export` writes directly to the `--output` path. Each request generates an `organization_export.requested` event and audit entry, and the end generates `organization_export.completed` or `organization_export.failed` (actor `system`, with `requested_by` in the payload).

> The file gathers the personal data of all records. Treat it like a backup: keep it in a safe place and delete unnecessary copies.

## Zip structure

```
manifest.json
config/package.json
config/keys.json
config/items/pipeline.jsonl
config/items/pipeline_stage.jsonl
config/items/lost_reason.jsonl
config/items/tag.jsonl
config/items/activity_type.jsonl
members.jsonl
records/<type>.jsonl              one per record type (company, contact, deal, activity, product, custom.<key>, types from other modules)
relations/contact_companies.jsonl
relations/deal_contacts.jsonl
relations/record_relations.jsonl
relations/taggings.jsonl
relations/activity_links.jsonl
relations/attachments.jsonl
data/activity_recurrences.jsonl
data/deal_items.jsonl
history/notes.jsonl
history/domain_events.jsonl
history/deal_stage_history.jsonl
files/files.jsonl
files/blobs/<file_id>             binary content of each attachment
```

`.jsonl` files are [JSON Lines](https://jsonlines.org/): one JSON object per line, UTF-8, with no comma between lines. Every record type and every dataset appears in the zip, even if empty.

### `manifest.json`

```json
{
  "format": "aphel.organization_export",
  "format_version": 1,
  "aphel_version": "0.1.0",
  "exported_at": "2026-10-22T00:49:35.873377507+00:00",
  "export_id": "01a16115-b611-718c-96db-08221a67b033",
  "organization": { "name": "Demo", "locale": "pt-BR", "timezone": "America/Sao_Paulo", "currency": "BRL" },
  "include_files": true,
  "counts": {
    "records": { "company": 20, "contact": 60, "deal": 40, "activity": 70, "product": 0, "custom.imovel": 3 },
    "datasets": { "notes": 25, "attachments": 3, "taggings": 6, "…": 0 },
    "members": 2, "files": 3, "file_bytes": 6212, "missing_files": 0, "archive_bytes": 0
  },
  "entries": [
    { "path": "records/contact.jsonl", "kind": "records", "name": "contact", "count": 60, "bytes": 51234, "sha256": "…" }
  ]
}
```

- `format` and `format_version` identify the format. A restore refuses versions greater than the one it knows; new versions only add fields or files.
- `entries` lists **all** the files in the zip (except the manifest itself) with `kind` (`config_package`, `config_keys`, `config_items`, `members`, `records`, `dataset`, `files`, `blob`), `name` (record type, dataset or configuration item), number of lines, size and SHA-256. The restore checks each checksum before writing anything.
- The manifest's `archive_bytes` is always `0` (the final size is only known after closing the zip); the actual size appears in the export summary in the API.

### Records (`records/<type>.jsonl`)

Each line:

```json
{
  "id": "01a160f6-08f5-75ca-84e9-a32726b3b526",
  "row": { "id": "…", "first_name": "Ana", "email": "ana@exemplo.com", "owner_id": "…", "company_id": "…", "archived_at": null, "deleted_at": null, "created_at": "2026-08-09T19:44:58.231513Z", "…": "…" },
  "custom_fields": { "preferred_channel": "phone", "aniversario": "1990-05-17" }
}
```

- `row` carries **all the columns** of the type's table, including archived and deleted (trash) records, so the restore reproduces the complete state. Dates and times in UTC (`YYYY-MM-DDTHH:MM:SS.ffffffZ`), dates without time as `YYYY-MM-DD`, money and decimals as text (`"15000.00"`), booleans as `0`/`1`, JSON as JSON.
- `custom_fields` uses the field's **stable key** (`fields[].key` from the configuration package); select comes as the option's `value`, multi-select as a list, user as the user id and related record as the record id.
- Custom object types appear as `custom.<key>` (table `custom_records`); the title also appears in `custom_fields.name`.
- References to users (`owner_id`, `created_by`, `author_id`…) are ids from the source installation; `members.jsonl` contains each of them.

### Configuration (`config/`)

- `package.json` is the organization's configuration package ([format](../config-packages/README.md)), with all registered sections: fields, pipelines, loss reasons, activity types, tags, layouts, organization views, dashboard, custom objects and relationships, automations and extension sections.
- `keys.json` maps the source id of each item referenced by the data to its stable key: `{ "pipeline": { "<id>": "vendas" }, "pipeline_stage": { "<id>": "vendas/proposta" }, "lost_reason": {…}, "tag": {…}, "activity_type": {…}, "relationship": {…} }`. Stages use `pipeline/stage`.
- `items/*.jsonl` carries the original rows of pipelines, stages, loss reasons, tags and activity types, **including inactive ones** (which the configuration package does not export), to recreate whatever is missing.

### Members (`members.jsonl`)

`{ "id", "email", "display_name", "role", "status", "password_hash", "locale" }` for each user of the source (`status` = `invited`, `active` or `deactivated`). The password hash (Argon2) goes along so users can sign in with the same password after the restore. Sessions, 2FA and tokens are **not** exported.

### Links, auxiliary data and history

Original table rows, in the same pattern as `row`:

| File | Content |
| --- | --- |
| `relations/contact_companies.jsonl` | Additional contact–company links (with role). The main link is in `contact.row.company_id` |
| `relations/deal_contacts.jsonl` | Additional contacts of deals |
| `relations/record_relations.jsonl` | Custom relationships (`relationship_id` + `from_id` → `to_id`) |
| `relations/taggings.jsonl` | Tags applied to records |
| `relations/activity_links.jsonl` | Extra records linked to activities |
| `relations/attachments.jsonl` | Attachment → record (`record_type`, `record_id`, `file_id`) |
| `data/activity_recurrences.jsonl` | Recurring task rules |
| `data/deal_items.jsonl` | Products and items of deals |
| `history/notes.jsonl` | Record notes |
| `history/domain_events.jsonl` | Domain events whose record is of an exported type (or a note): they feed the timeline |
| `history/deal_stage_history.jsonl` | Stage history of deals (conversion reports) |

### Files (`files/`)

`files/files.jsonl` describes each file **attached to some record** (`id`, `original_name`, `mime_type`, `size`, `checksum` SHA-256, `uploaded_by`, `created_at`, `deleted_at`) and `path`, the path of the content in the zip (`files/blobs/<id>`), or `null` when the export was requested without files or the file could not be read (counted in `missing_files`). Files without an attachment (previous exports, logo, avatars) are left out.

## What is not included

Sessions, 2FA, API tokens, integration and webhook secrets (they never leave the vault), audit log and security events, notifications, personal preferences, favorites, reminders, goals, private views, queues and jobs, extension data that is not a record type, import and export history.

## Restore (`aphel org import` or installer)

```bash
aphel org import file.zip
```

From the screen, the installer offers **Restore from a backup** (API: `POST /api/v1/installer/restore`, multipart with the `file` field, available only while the installer is available; it responds `201 { "data": { "restored": true } }` and the screen sends you to the login).

Limits (fixed in the code):

| Limit | Value | Where |
| --- | --- | --- |
| Upload size | 2 GiB (`413 payload_too_large`) | installer only; larger backups go through `aphel org import` |
| Attempts | 5 per hour per IP and 20 per hour in total (`429 rate_limited`) | installer only |
| Zip entries | 500,000 | installer and CLI |
| Uncompressed size | 4 GiB per entry, 64 GiB in total (based on the sizes declared in the zip; reading beyond the declared size is refused) | installer and CLI |
| JSON line | 64 MiB | installer and CLI |
| JSON read whole (manifest, package, keys) | 256 MiB | installer and CLI |

If the restore fails after creating the installation, whatever was written is undone on a dedicated connection (with `FOREIGN_KEY_CHECKS = 0`), which is closed at the end instead of going back to the pool.

1. **Target:** an **empty** installation, not installed yet. If the installation has already been done or has records, the restore stops with `organization_not_empty` (409).
2. **Verification:** opens the zip, checks the format, version and the SHA-256 of each manifest entry (`invalid_organization_export` if anything differs). The backup must have an active administrator with a password.
3. **Installation:** creates the organization with the manifest's name, language, time zone and currency and the backup's first active administrator (same id, email and password). The restore runs with their access.
4. **Configuration:** applies `config/package.json` with the package engine (plan + apply, resolvable conflicts applied). This step is a transaction of its own.
5. **Configuration id mapping:** each pipeline, stage, loss reason, tag, activity type and relationship is found in the target by the key from `keys.json`. Items that do not exist in the target (usually inactive ones) are recreated from `config/items/`.
6. **Users:** the users from `members.jsonl` are recreated with the same id, email, name, role (by key; an unknown role becomes `member`), status, language and password hash. Those that could not be created (invalid email) become empty references and are listed in the report (`unmapped_members`).
7. **Data ids:** the original ids are preserved.
8. **Data:** in a single transaction, it writes files (content rewritten to the target storage, with the checksum verified), recurrences, records (companies, contacts, products, custom objects, deals, activities, types from other modules), custom field values by key, links, notes, timeline events (written as already dispatched, without triggering automations, webhooks or notifications) and stage history. It finishes with the `organization.restored` event and audit entry. If anything fails, none of the data is written and the files already copied are removed.
9. **Search:** rebuilds the global search index.

The command prints a JSON report with counts per type and dataset, files, custom values, recreated configuration items, mapped and unmapped members, skipped lines and warnings.

### Known limitations

- Custom fields that do not exist in the target after applying the package (for example, inactive in the source) are ignored with a warning.
- Automation actions that depend on secrets (webhooks, integrations) come back without the secret and must be configured again.
- Team-based pipeline owners and goals are not restored.
- The JSON Lines files are not loaded whole into memory: each one is decompressed to a temporary file and read line by line (one pass to check the ids, another to write); only the ids stay in memory. Attachments are also extracted to a temporary file and sent to the storage by streaming. Reserve space in the temporary directory the size of the largest uncompressed entry.
