# Trash

`trash` module (`back/modules/trash`, web in `web/src/modules/trash`). Lists, restores and permanently deletes soft-deleted records (`deleted_at`) of every type that has a title (contacts, companies, deals, activities, quotes, custom objects…). Retention (1–365 days, default 30) lives in `trash_settings` and the `trash.purge` job (daily) deletes what has expired.

## Access

Every route requires `settings.manage` and, in addition, respects each type's record scope:

- **List and types:** only the types the user can read are included, and for each type only the records within the read permission's `record_filter` (own, team or organization, plus shares), applied in the query with `push_record_scope_as`.
- **Restore and permanently delete:** the record must be visible through read access (otherwise 404 `trash_item_not_found`) and within the scope of the type's delete permission (`<resource>.delete`, or the update/read one when the type has no delete permission; otherwise 403 `permission_denied`).
- The retention job runs with `AccessScope::system()`.

Records merged into another one do not appear in the trash and cannot be restored or deleted through it (their id redirects to the primary record); the retention job still deletes them when they expire. The trash does not know about the `duplicates` module: modules hide records by contributing `trash::TrashExclusion` (`app.contribute::<dyn TrashExclusion>(..)`), which adds an `AND` condition to each type's query; `duplicates` contributes the exclusion by `record_merges`.

## Restore

`POST /api/v1/trash/{tipo}/{id}/restore` clears `deleted_at` and goes through the record pipeline as an update write with the change `deleted_at → null`: `RecordValidating` (validation rules, approvals and the other vetoes can block it), `RecordSaved`, the `<type>.restored` event with audit in the same transaction and `RecordCommitted` after the commit. Between validation and `RecordSaved`, `pipeline.restored` runs (`RecordRestored` hook); `leads` uses this hook to reopen the stage history.

## Permanently delete

`DELETE /api/v1/trash/{tipo}/{id}` (and the retention job) calls `CustomFieldWriter::purge_record`, which runs the `RecordPurged` hook and deletes custom values and `record_relations`. Each module removes what it stores for the record in its `RecordPurged` handler (`notes`, `tags`, `activities`, `duplicates`). The pipeline itself (kernel) deletes the attachment links (and the file rows left without references) and the shares, and clears the record-type fields that pointed to it, so privacy and retention have the same effect. The trash only deletes the record's row, publishes `<type>.purged` with audit and, after the commit, removes from storage the files returned in `PurgedRecord::orphan_storage_keys`.
