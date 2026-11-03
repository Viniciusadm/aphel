# Global search

Global search (`GET /api/v1/search`, the `Ctrl+K` palette and the `/search` page) searches all types in the `RecordTypeRegistry` at once and returns the results grouped by type. It uses its own index, derived from the records, which is updated by the worker and can be rebuilt at any time.

## Pieces

| Piece | Where it lives | What it does |
| --- | --- | --- |
| `parse_terms`, `normalize_content` | `back/kernel/src/application/search/text.rs` | Normalization of the query and the content (lowercase, no accents, phone digits only) |
| `indexed_fields`, `build_document`, `compose_display` | `back/kernel/src/application/search/document.rs` | Which fields of a type go into the index, each record's document and the choice of title/subtitle |
| `SearchUseCases` | `back/kernel/src/application/search/use_cases.rs` | Query (`search`) and indexing (`reindex_records`, `reindex_type`, `reindex_organization`) |
| `SearchIndexConsumer` (`search.index`) and `SearchReindexHandler` (`search.reindex`) | `back/kernel/src/application/search/worker.rs` | Incremental update from events and batch rebuild in the worker |
| `MySqlSearchIndex`, `MySqlSearchSource` | `back/kernel/src/infrastructure/search` | Index SQL (FULLTEXT) and generic reading of the modules' tables |
| `aphel search reindex` | `back/app/src/cli/search.rs` | Rebuild from the command line |
| Page and components | `web/src/features/search` | `SearchPage` (`/search?q=`), `SearchResultGroups`, `searchHitPath`, `useSearchRecordTypeLabel`, `useGlobalSearch` |

## Tables

- **`search_index`** (migration `1510`): one row per indexed record, key `(record_type, record_id)`, with `owner_id` (for the own/team scope), `title`/`title_field`, `subtitle`/`subtitle_field` and `indexed_at`.
- **`search_index_terms`** (migration `1511`): one row per indexed field of the record, key `(record_type, record_id, field_key)`, with `value` (displayable text, up to 1000 characters), `content` (normalized text, `FULLTEXT ... WITH PARSER ngram`), `position`, `weight` (3 for the title field, 2 for the other system fields, 1 for custom ones) and `is_title_candidate`. Cascades on delete from `search_index`.

The FULLTEXT index is created with `innodb_ft_enable_stopword = OFF`: with the n-gram parser, InnoDB's default stopword list would discard bigrams common in Portuguese (`de`, `an`, `la`, `com`…). The content is stored without accents and in lowercase (`normalize_name` from `modules/duplicates/domain`), because FULLTEXT compares tokens without taking the collation into account; the query goes through the same normalization.

Storing the text per field (rather than in a single column with everything) is what makes it possible to respect field permissions, which depend on the searcher's role and therefore cannot be decided at indexing time.

## What is indexed

For each type in the registry (including new types, such as `product` and custom modules, with no specific code):

- system fields marked `searchable()` in the type definition;
- active custom fields with `is_searchable`;
- in both cases, only text, long text, email, phone and URL types, and **never** `is_sensitive` fields (nor system fields marked `sensitive()`).

Deleted (`deleted_at`), archived (`archived_at`) and merged records (the duplicate is deleted on merge) leave the index; records without any indexable value are not kept in the index either.

**Aggregated content from other modules:** besides the fields, the record's document receives texts contributed by modules that implement `SearchContentSource` (`back/kernel/src/application/search/contents.rs`) and register with `app.contribute::<dyn SearchContentSource>(..)`. Each source becomes a `search_index_terms` row with key `@<key>` (weight 1, never title or subtitle):

| Source | Module | Key | What goes in | Who can see it |
| --- | --- | --- | --- | --- |
| `NoteSearchContent` | `notes` | `@notes` | Body of the record's team notes (up to 50, pinned and most recent first; private and deleted notes are left out; mentions become `@name`) | Whoever has `notes.read` |
| `TagSearchContent` | `tags` | `@tags` | Names of the record's tags | Whoever can read the type |

At query time, the `@<key>` key only goes into the `MATCH` if the role has the source's read permission (`read_permission`). The source also says which events affect the index (`listens_to`/`affected`): `note.created|updated|deleted` reindex the owning record (read from the payload's `record_type`/`record_id`), `record.tagged|untagged` reindex the record, `tag.updated` reindexes the records with the tag and `tag.deleted` enqueues a general `search.reindex` (the taggings have already been deleted). The batch rebuild (`search.reindex`, `aphel search reindex`) reads the sources together with the fields.

Phone numbers (E.164, `+5511999998888`) are stored as digits only (`5511999998888`). Since n-grams match any contiguous substring, `11999998888`, `999998888` and `5511999998888` all find the same number. Phone-like text in text fields also gets a digits-only version.

**Title and subtitle**: the title is the first indexed system text field with a value (in definition order: `name` for people and organizations, `title` for deals and activities); the subtitle is the next indexed field with a value that is not contained in the title (for people, the email or phone; for organizations, the domain).

## Query

1. `q` has 2 to 100 characters. It is normalized and split into words of at least 2 characters (at most 8). A phone-like term (only digits, spaces and `+()-./`, with 6 to 15 digits) becomes a single term with the digits.
2. For each type (those in `types`, in the requested order, or all of them): if the caller lacks the type's read permission, the type is skipped (it does not appear, no error).
3. `FieldSet::for_scope` defines the fields visible to the role; only indexed and visible fields go into the `MATCH` (`field_key IN (...)`). Every word must appear in some visible field of the record (`HAVING MAX(MATCH(...)) > 0` per word). Relevance is the sum of `MATCH * weight`.
4. The type's scope (all, own or team) is already applied in the index through `owner_id`, with the same function as the query compiler (`push_record_scope_as`).
5. The first up to 100 candidates are **revalidated** by a `QueryPlan` with `id IN (...)` run by the `RecordQueryExecutor`, the same one used by listings: deletion, archiving and own/team scope apply exactly as in the module's listing. For deals, the per-team pipeline restriction (`pipeline_restriction`/`restrict_to_pipelines`) is applied as well. That way an outdated index never shows a record the user could not open.
6. `title`/`subtitle` come from the index header when the source fields are visible; if any of them is hidden from the role, they are recomputed from the values of the visible fields (and fall back to the record id if there are none).

`total` counts the visible results among the type's first 100 candidates; `limit` (default 5, maximum 20) is per type. Types without results are omitted. Without `types`, the groups are sorted by best relevance.

## Updating

- **Incremental:** the `search.index` consumer accepts every `<type>.*` event whose prefix is a registered type (`contact.created`, `contact.updated`, `deal.stage_changed`, `company.archived`, `contact.restored`, `contact.purged`, `contact.anonymized`…) and reindexes the event's record by reading its current state (idempotent; repeated deliveries cause no problem). On `contact.merged`/`company.merged` it also reindexes the `merged_id` (which leaves the index). Custom field changes arrive in the record's `<type>.updated` event.
- **Field change:** `field.updated`, `field.activated` and `field.deactivated` (for example, marking a field as sensitive or not searchable) enqueue a `search.reindex` for that type only.
- **Batch:** the `search.reindex` job (`{ record_type?, cursor? }`) walks the records in batches of 200 by `id` and, when it finishes a type, deletes from the index whatever was not revisited since the start of the pass (physically removed records). Each job works for up to 240 s and, if it does not finish, enqueues the continuation with the cursor.
- **Initial load:** migration `1512` enqueues a general `search.reindex`, so a database that already had data is indexed as soon as the worker runs. There is no reindexing on `serve` boot.

The index is derived data: indexing publishes no event and writes no audit entry.

## Reindexing manually

```bash
aphel search reindex                                   # all types
aphel search reindex --organization acme               # one organization
aphel search reindex --organization acme --record-type contact
```

The command runs in the process itself (no worker), is idempotent and can be repeated.

## New record types

A module that wants to add text to the document of records of other types (comments, attachments…) contributes a `SearchContentSource` instead of becoming a record type.

A type registered in the `RecordTypeRegistry` joins the search on its own: just mark the searchable system fields with `searchable()` (the first text field becomes the title) and publish `<type>.*` events with `with_record`. For the link in the UI, the module registers the resource with `registerRecordResource({ recordType, detailPath, ... })`; the type's label lives in `web/src/i18n/<locale>/search.json` (`recordTypes`/`recordTypesPlural`, with the type itself as fallback). If the type has a visibility rule beyond the owner scope, it must be applied in the revalidation (like the restricted pipelines of deals).
