# Duplicates and merging

The `duplicates` module (`back/modules/duplicates`, web in `web/src/modules/duplicates`): detection rules per type, candidate queue (`duplicate_candidates`), merging and redirection of merged ids (`record_merges`).

## Detection

- `POST /duplicates/check` and the post-creation check (`DuplicateDetector`) look for similar records. In the user's check, the read scope goes into the query itself before the limit, so that invisible records do not take up the slots (at most 10 results). After creation, the candidates written to the queue are calculated without a scope (installation-wide queue) and the response shows only the visible ones.
- The `duplicates.scan` job (daily, 03:15) scans all records and writes `pending` candidates.
- The `duplicates.cleanup` job (daily, 02:45) re-evaluates the `pending` candidates in batches: it removes those where one of the records is deleted or nonexistent and those that no longer match the enabled rules; it updates the rules of those that still match. It is idempotent; types without an enabled rule are left as they are. Dismissed and merged candidates do not change.
- `GET /duplicates/{id}` returns the candidate with its status (`pending`, `dismissed` or `merged`); for a merged one, the losing record (already in the trash) also counts as visible.

## Merging

`POST /duplicates/merge` accepts `If-Match` with the primary record's `updated_at` (409 `version_conflict` when it has changed; the versions of both records are also rechecked with `FOR UPDATE` inside the transaction). Everything in one transaction:

1. The loser's links (tasks, notes, attachments, tags, relationships, references from record-type fields and the type's own links) move to the primary (organizations: the loser's child organizations move to the primary first); `record_merges` stores the redirection and the pair's candidates become `merged`.
2. The loser is deleted through the pipeline: `RecordDeleting` (`restrict` relationships can block), soft deletion, `RecordDeleted` and the `<type>.deleted` event with `merged_into`.
3. The chosen values are written to the primary (system and custom; hidden fields only fill empty values). Since the loser is already deleted in the same transaction and the uniqueness check ignores deleted records, the value of an `is_unique` field can come from the loser.
4. If something changed on the primary, the update goes through the pipeline: `RecordValidating` (validation rules, approvals…), `RecordSaved`, the `<type>.updated` event and `RecordCommitted` after the commit.
5. The `<type>.merged` event with what was moved and the changes.

### Restoring the loser

The loser **keeps its own custom values** (they are not erased by the merge) until the permanent purge, so a restore brings those values back. The moved links **stay with the primary**: restoring the loser does not bring them back. The trash does not list merged records (the id redirects to the primary; the module contributes `trash::TrashExclusion` with the condition on `record_merges`), so this restore only happens through paths that ignore the redirection.

## Purge

When a record is permanently deleted, the module's `RecordPurged` handler removes the candidates it appears in and the redirections where it was the primary.
