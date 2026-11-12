---
title: Records API
description: The conventions of record endpoints in Aphel, covering the generic entity routes, per-module routes, custom objects, custom fields, relationships, versions, archiving, deletion, bulk operations and file attachments.
sidebar:
  order: 5
---

Everything business-related in Aphel is a **record** of some **record type**: a person, an organization, a deal, a task, a note, a record of a custom object. There are three families of routes to work with records, and they share the same conventions for authentication, errors, [pagination and filters](/_aphel/docs/reference/pagination-and-filters/), versions and events.

| Family | Routes | Use for |
| --- | --- | --- |
| Per-module routes | `/api/v1/people`, `/api/v1/organizations`, `/api/v1/deals`, `/api/v1/tasks`, ... | The first-class API of each module. Typed request and response bodies, extra actions (move a deal, mark it won). The OpenAPI document describes each of them. |
| Custom objects | `/api/v1/objects/{key}/records` | Records of objects an administrator defined, or that a plugin declared. Same shape for every object. |
| Generic entity routes | `/api/v1/entities/{record_type}/records` | One uniform CRUD surface over any registered record type that opts in. |

## Record types

A record type key has the form `<module>.<entity>`, for example `people.person`, `organizations.organization`, `deals.deal`, `tasks.task`, `notes.note`, `quotes.quote`, `catalog_items.item`. Custom objects use `custom.<key>`, where `<key>` is the key given to the object when it was created. These keys appear in permissions (`people.read`), events, attachments and the generic routes.

`GET /api/v1/entities` lists the record types the caller can read or create, with their `key`, `record_type`, `storage`, `title_field`, `labels`, `traits`, `indexes`, `references`, `bulk_fields` and the permission keys used for `read`, `create`, `update`, `archive`, `delete` and `export`. `GET /api/v1/fields/{record_type}/schema` describes the fields of one type, and `GET /api/v1/objects/{key}/schema` describes a custom object, including its relations and events.

## Per-module routes

Modules follow one convention. The examples use people; organizations, deals and the other record types are analogous.

| Operation | Route | Success |
| --- | --- | --- |
| List | `GET /api/v1/people` | `200` with `{ data: [...], meta }` |
| Create | `POST /api/v1/people` | `201` with `{ data: record }` |
| Read | `GET /api/v1/people/{id}` | `200` with `{ data: record }` |
| Update | `PATCH /api/v1/people/{id}` | `200` with `{ data: record }` |
| Archive | `POST /api/v1/people/{id}/archive` | `200` with `{ data: record }` |
| Unarchive | `POST /api/v1/people/{id}/unarchive` | `200` with `{ data: record }` |
| Delete | `DELETE /api/v1/people/{id}` | `204`, the record moves to the trash |

Module-specific actions sit next to these, for example `POST /api/v1/deals/{id}/won`, `.../lost`, `.../reopen`, `.../move`, `.../move-pipeline`, `GET /api/v1/deals/board` and `POST /api/v1/people/{id}/organizations`. Each route requires the permission shown in its OpenAPI description (`people.read`, `people.create`, `people.update`, `people.delete`, `people.archive`), applied within the record scope of the caller (own, team or organization).

### Create

```bash
curl -X POST "$APHEL_URL/api/v1/people" \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: 4e7c5f0a-52d1-4d0e-8d10-2a61a0a8b001" \
  -d '{
    "first_name": "Ana",
    "last_name": "Souza",
    "email": "Ana@Acme.com.br",
    "phone": "(11) 3333-4444",
    "company_id": "0192a1b2-0000-7000-8000-0000000000c1",
    "custom_fields": { "lead_source": "website" }
  }'
```

The response is `201`:

```json
{
  "data": {
    "id": "0192a1b2-0000-7000-8000-000000000001",
    "name": "Ana Souza",
    "first_name": "Ana",
    "last_name": "Souza",
    "email": "ana@acme.com.br",
    "phone": "+551133334444",
    "mobile": null,
    "job_title": null,
    "company_id": "0192a1b2-0000-7000-8000-0000000000c1",
    "company": { "id": "0192a1b2-0000-7000-8000-0000000000c1", "name": "Acme Ltda", "domain": "acme.com.br" },
    "owner_id": "0192a1b2-0000-7000-8000-0000000000aa",
    "owner": { "id": "0192a1b2-0000-7000-8000-0000000000aa", "display_name": "Bruna Lima" },
    "interaction_count": 0,
    "archived_at": null,
    "created_at": "2026-10-20T13:45:00Z",
    "updated_at": "2026-10-20T13:45:00.123456Z",
    "custom_fields": { "lead_source": "website" },
    "possible_duplicates": []
  }
}
```

What the example shows:

- The server normalizes values: the email is lower-cased, the phone becomes E.164, and `name` is derived from the first and last names.
- `owner_id` defaults to the caller. Records with owners are limited by the record scope of each role.
- `possible_duplicates` appears only in the create response. It lists visible people the duplicate rules match; it is a warning and never an error, and the pairs enter the duplicate queue.
- The response is abbreviated here. The exact schema of each record type is in the OpenAPI document (for example `ContactRequest` and `ContactResponse`), and `company_id` and `lead_source` in this example stand for an existing organization and a custom field that exists in your installation.

### Update and versions

`PATCH` accepts any subset of the fields of the create request. Properties you omit stay unchanged, `null` clears an optional property and `custom_fields` only touches the keys sent.

```bash
curl -X PATCH "$APHEL_URL/api/v1/people/0192a1b2-0000-7000-8000-000000000001" \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -H "Content-Type: application/json" \
  -H "If-Match: 2026-10-20T13:45:00.123456Z" \
  -d '{"job_title":"Head of Sales","custom_fields":{"lead_source":null}}'
```

The version of a record is its `updated_at`. Send the value you read in `If-Match` to make the update conditional: if the record changed meanwhile the answer is `409 version_conflict` and nothing is written. Without the header the update is unconditional. The check runs again inside the write transaction while the row is locked, so concurrent updates cannot both pass.

On the generic entity routes, `If-Match` works for record types whose handler supports durable (single-transaction) updates; others answer `422 record_operation_not_supported` when the header is present.

### Archive, delete and restore

| Action | Effect |
| --- | --- |
| Archive (`POST .../archive`) | Hides the record from default listings. `archived=include` or `archived=only` brings it back. The record keeps its links. Publishes `<type>.archived`. |
| Unarchive (`POST .../unarchive`) | Archive undone. Publishes `<type>.unarchived`. |
| Delete (`DELETE ...`) | Soft delete: the record goes to the trash and publishes `<type>.deleted`. Relationships configured with `on_delete: restrict` block the deletion with `409 record_has_relations` or `409 record_referenced`. |
| Restore (`POST /api/v1/trash/{record_type}/{record_id}/restore`) | Brings a deleted record back through the normal record pipeline, including validation rules and approvals. Needs `settings.manage`. |
| Purge (`DELETE /api/v1/trash/{record_type}/{record_id}`) | Deletes for good and publishes `<type>.purged`. Also needs `settings.manage`. |

`GET /api/v1/trash` lists deleted records and `GET /api/v1/trash/types` lists the record types the trash handles; the record type names these routes accept are the ones that listing returns. Expired trash items are purged automatically according to the retention in `GET /api/v1/trash/settings`.

## Custom fields

Custom fields are typed values stored next to the system fields. They are read and written in the `custom_fields` object of the record, keyed by field key:

```json
{
  "custom_fields": {
    "budget": "15000.00",
    "segment": "enterprise",
    "interests": ["crm", "email"],
    "renewal_date": "2027-03-01",
    "contract_signed": true,
    "account_manager": "0192a1b2-0000-7000-8000-0000000000aa"
  }
}
```

| Field type | JSON value |
| --- | --- |
| `text`, `long_text`, `url`, `email`, `phone` | String. Emails are lower-cased and phones normalized to E.164. |
| `number`, `currency` | Decimal number or string, such as `"15000.00"`. Responses return a decimal. |
| `date` | `YYYY-MM-DD` |
| `datetime` | RFC 3339 timestamp |
| `checkbox` | `true` or `false` |
| `select` | The `value` of an option |
| `multi_select` | Array of option values |
| `user` | A user id |
| `record` | The id of a record of the target type; the caller needs read access to it |
| `file`, `image` | The id of a file attached to the same record; `image` also needs an image type |
| `document` | A string with valid JSON |

Module-registered field types (and formula fields, which are computed and read-only) behave like the native type they are stored as. Rules:

- Sending `null` clears a value. A required field cannot be cleared and answers `422`.
- Unknown keys, inactive fields and fields the role cannot write answer `422` with the path `custom_fields.<key>`. Hidden fields answer `field_not_accessible`, read-only fields `field_read_only`.
- Responses omit fields the role cannot see.
- Field definitions are managed with `GET`/`POST /api/v1/fields/{record_type}` and the routes under it. The schema routes return the keys, types, options and filter operators to use.

## Custom objects

An administrator creates a custom object with `POST /api/v1/custom-objects` (permission `settings.manage`). Its `key` is immutable and becomes:

| Derived name | Value |
| --- | --- |
| Record type | `custom.<key>` |
| Records path | `/api/v1/objects/<key>/records` |
| Permissions | `custom.<key>.read`, `.create`, `.update`, `.delete`, `.export`, `.archive` |
| Events | `custom.<key>.created`, `.updated`, `.deleted`, `.archived`, `.unarchived` |

Routes:

| Route | Purpose |
| --- | --- |
| `GET /api/v1/objects` | Active objects the caller can read. |
| `GET /api/v1/objects/{key}/schema` | Fields, options, operators, relations and events as the caller sees them. |
| `GET /api/v1/objects/{key}/records` | List with the full query language. |
| `POST /api/v1/objects/{key}/records` | Create. |
| `GET`, `PATCH`, `DELETE /api/v1/objects/{key}/records/{record_id}` | Read, update, delete. |
| `POST .../records/{record_id}/archive` and `.../unarchive` | Archive and restore from the archive (permission `custom.<key>.archive`). |

Creating a record:

```bash
curl -X POST "$APHEL_URL/api/v1/objects/contract/records" \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "owner_id": "0192a1b2-0000-7000-8000-0000000000aa",
    "custom_fields": { "title": "Contract 2026-001", "value": "12000.00" },
    "relations": { "company": "0192a1b2-0000-7000-8000-0000000000c1" }
  }'
```

The body has three optional properties: `custom_fields`, `owner_id` (defaults to the caller when the object has owners) and `relations`. The record `title` comes from the field the object uses as its title field. The response is:

```json
{
  "data": {
    "id": "0192a1b2-0000-7000-8000-000000000042",
    "record_type": "custom.contract",
    "title": "Contract 2026-001",
    "owner_id": "0192a1b2-0000-7000-8000-0000000000aa",
    "custom_fields": { "title": "Contract 2026-001", "value": "12000.00" },
    "archived_at": null,
    "created_by": "0192a1b2-0000-7000-8000-0000000000aa",
    "created_at": "2026-10-20T13:45:00Z",
    "updated_at": "2026-10-20T13:45:00.123456Z"
  }
}
```

The key `company` in `relations` must be a relation defined for the object (see its schema). `PATCH` takes the same body and `If-Match`. Links are changed through the relation routes after creation. Unknown or inactive objects answer `404 custom_object_not_found`; missing or out-of-scope records `404 custom_record_not_found`.

## Generic entity routes

These routes work over the registry of record types, so a client can handle any type without knowing its module:

| Route | Purpose |
| --- | --- |
| `GET /api/v1/entities` | The record types you can use, with their permissions. |
| `GET /api/v1/entities/{record_type}/records` | List with the full query language. |
| `POST /api/v1/entities/{record_type}/records` | Create. |
| `GET /api/v1/entities/{record_type}/records/{id}` | Read. |
| `PATCH /api/v1/entities/{record_type}/records/{id}` | Update; supports `If-Match`. |
| `DELETE /api/v1/entities/{record_type}/records/{id}` | Delete. |

`{record_type}` is the key from `GET /api/v1/entities` (for example `people.person`). Behavior:

- Reads work for every registered type under its read permission and record scope.
- Writes work only for types that opted in to generic mutations. Other types answer `422 record_operation_not_supported`, and you should use the module routes instead.
- The body of a create or update is a flat JSON object of the system fields of the type plus `custom_fields`. System fields that are unknown or read-only answer `422` with the field name. Required fields missing on create answer `422` (`is required`).
- The response of a read is the field values plus `id`, `record_type`, `entity`, `archived_at` (when the type can be archived) and `custom_fields`.
- Create, update and delete publish the same `<type>.created|updated|deleted` events as the module routes, and run the same validation, approval and duplicate hooks.
- This family does not expose archive and unarchive; use the module routes or the bulk operations below.

## Relationships

Relationships between record types are defined by administrators (and by modules) and apply to core and custom records alike.

| Route | Purpose |
| --- | --- |
| `GET /api/v1/records/{record_type}/{record_id}/relations` | The relations of the record and how many linked records you can see. |
| `GET /api/v1/records/{record_type}/{record_id}/relations/{relation}` | The linked records, newest link first. |
| `POST /api/v1/records/{record_type}/{record_id}/relations/{relation}` | Link, with `{ "record_id": "..." }`. Linking twice is a no-op and answers `linked: false`. |
| `DELETE /api/v1/records/{record_type}/{record_id}/relations/{relation}/{target_id}` | Unlink. |

Linking needs update permission on the record and read access to the other one. Cardinality is enforced (`409 relation_limit_reached`), the last link of a required relation cannot be removed, and link changes publish `record.linked` and `record.unlinked`. You can also filter by a related record's fields (`company.industry`) and embed related records with `include`; see [Pagination and filters](/_aphel/docs/reference/pagination-and-filters/). People and organizations have their own link routes (`/people/{id}/organizations`).

## Sharing

A record can be shared with a user or a team on top of the role scopes. `POST /api/v1/records/{record_type}/{record_id}/shares` takes `{ "grantee_type": "user" or "team", "grantee_id": "...", "access": "read" or "edit" }`. Only the owner of the record or someone with `records.share` can do it, at most 100 shares exist per record, and a share never allows deleting or sharing again. `GET` lists the shares, `PATCH .../shares/{share_id}` changes the access and `DELETE` removes it.

## Bulk operations

`POST /api/v1/bulk-operations` applies one action to many records of one type.

```bash
curl -X POST "$APHEL_URL/api/v1/bulk-operations" \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "record_type": "people.person",
    "action": { "action": "assign_owner", "owner_id": "0192a1b2-0000-7000-8000-0000000000aa" },
    "selection": { "query": { "filter": { "op": "and", "conditions": [ { "field": "city", "operator": "eq", "value": "Recife" } ] }, "archived": "exclude" } }
  }'
```

- `selection` is either `{ "ids": [...] }` (up to 10 000 ids) or `{ "query": { "filter", "search", "archived" } }`. A selection of more than 10 000 records answers `422 bulk_too_large`; an empty one `422 bulk_empty_selection`.
- Actions: `assign_owner` (`owner_id`), `set_field` (`field`, `value`), `add_tag` and `remove_tag` (`tag_id`), `move_stage` (`stage_id`), `enroll_in_sequence` (`sequence_id`), `archive`, `unarchive` and `delete`. Which are available depends on the record type; an unsupported pair answers `422 bulk_action_not_supported`.
- Every record goes through the same use case as the single action, so validation, permissions, events and audit are the same. Records the caller cannot change are `skipped`.
- Up to 200 records run immediately and the answer is `200` with the finished operation. More than 200 are queued: the answer is `202` with `background: true`, and you poll `GET /api/v1/bulk-operations/{id}` until `status` is final. `GET /api/v1/bulk-operations/{id}/items` reports skipped and failed records with the error code of each.
- `POST /api/v1/bulk-operations/export` queues an export of a selection (selected ids are limited to 100; use a filter for more). Follow it in `GET /api/v1/exports/{id}`.

Operations are listed with `GET /api/v1/bulk-operations`; each shows `total`, `processed`, `succeeded`, `skipped` and `failed`.

## File attachments

Files are stored once and attached to records. Downloads always go through an authenticated route.

**Upload** with `multipart/form-data`. The part `file` is required. To attach at upload time, send `record_type` and `record_id` together as text parts.

```bash
curl -X POST "$APHEL_URL/api/v1/files" \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -F "file=@proposal.pdf" \
  -F "record_type=deals.deal" \
  -F "record_id=0192a1b2-0000-7000-8000-0000000000d1"
```

The `201` response has the stored file and, when attached, the attachment:

```json
{
  "data": {
    "file": {
      "id": "0192a1b2-0000-7000-8000-0000000000f1",
      "original_name": "proposal.pdf",
      "mime_type": "application/pdf",
      "size": 48213,
      "checksum": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
      "uploaded_by": { "id": "0192a1b2-0000-7000-8000-0000000000aa", "display_name": "Bruna Lima" },
      "created_at": "2026-10-20T13:45:00Z",
      "download_url": "/api/v1/files/0192a1b2-0000-7000-8000-0000000000f1"
    },
    "attachment": {
      "id": "0192a1b2-0000-7000-8000-0000000000a1",
      "record_type": "deals.deal",
      "record_id": "0192a1b2-0000-7000-8000-0000000000d1",
      "file": { "id": "0192a1b2-0000-7000-8000-0000000000f1" },
      "created_by": { "id": "0192a1b2-0000-7000-8000-0000000000aa", "display_name": "Bruna Lima" },
      "created_at": "2026-10-20T13:45:00Z"
    }
  }
}
```

| Route | Purpose |
| --- | --- |
| `POST /api/v1/files` | Upload, optionally attaching. Attaching needs update permission on the record. |
| `GET /api/v1/files/{file_id}` | Download. Served as an attachment with the detected content type, `X-Content-Type-Options: nosniff` and a restrictive content security policy. |
| `GET /api/v1/attachments?record_type=...&record_id=...` | Attachments of a record, newest first, paginated. Needs read permission on the record. |
| `POST /api/v1/attachments` | Attach an existing file: `{ "file_id", "record_type", "record_id" }`. `409 already_attached` when it already is. |
| `DELETE /api/v1/attachments/{attachment_id}` | Detach. |

Rules:

- The size limit is the organization setting `max_upload_mb` (25 MB by default, up to 4096 MB). A larger file answers `413 file_too_large`; an empty one `422 empty_file`.
- The type is detected from the content, not the file name. Allowed types: JPEG, PNG, GIF, WebP, BMP, TIFF, HEIF and AVIF images; PDF; plain text, CSV and RTF; Microsoft Word, Excel and PowerPoint (legacy and Open XML) and OpenDocument text, spreadsheet and presentation files; ZIP; MP3 audio and MP4 video. Anything else, including HTML and other markup, answers `422 file_type_not_allowed`.
- The part `record_type` and `record_id` must be sent together, and the record type must support attachments (`422 unsupported_record_type` otherwise).
- Files that belong to organization exports and privacy packages cannot be downloaded from `/files`; they have their own download routes.

## Events

Every create, update, delete and archive publishes a domain event and an audit entry in the same transaction as the change. The public subset can be delivered to your systems; see [Webhooks and events](/_aphel/docs/reference/webhooks-and-events/).
