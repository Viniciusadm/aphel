# Company hierarchy

Companies form groups (parent company and subsidiaries) through the `companies.parent_company_id` column (migration `2600`, FK `ON DELETE SET NULL`). The `parent_company_id` system field is a `record` pointing to `company`, so the form, inline editing, import and the API use the generic record picker. There is no dedicated table or event: changing the parent is an edit of the company and goes out as `company.updated` with `changes`.

Code: `back/modules/companies/src/hierarchy` (session 31): `mod.rs` (validation and reparenting), `service.rs` (tree and totals over `&Ctx`), `store.rs` (`HierarchyStore`, `WITH RECURSIVE` queries) and `http.rs` (routes). The `parent_company_id` column is in the `companies` module migration. Web in `web/src/features/companies/hierarchy`.

## Write rules

`check_parent` runs inside the create and edit transaction (only when `parent_company_id` changes and is not null), against the stored state:

| Situation | Response |
| --- | --- |
| Parent does not exist, is deleted or is outside the writer's read scope (`companies.read`) | 422 `validation_error` on `parent_company_id` |
| Parent is the company itself or one of its descendants (at any level) | 422 `company_hierarchy_cycle` on `parent_company_id` |
| Parent depth + 1 + height of the moved company's subtree > `MAX_HIERARCHY_DEPTH` (10 levels, the root is level 1) | 422 `company_hierarchy_too_deep` on `parent_company_id` |

The lineage (`lineage`, from the company up to the root) and the subtree (`subtree`) are recursive queries filtered by `deleted_at IS NULL`, capped at 64 steps (`MAX_GROUP_RECURSION`) so they never loop on inconsistent data. When validating a new parent (`check_parent`), the edited company and the parent's lineage are re-read with `SELECT .. FOR UPDATE` row by row (`locked_lineage`) and the subtree height with `FOR SHARE` (`locked_height`), so two concurrent edits that cross the same branches are serialized (in the worst case one of them fails with a deadlock and can be retried).

## Deletion and merge

- **Deletion** (to the trash): in the same transaction, the direct subsidiaries of the deleted company are pointed to its parent (or are left without a parent if it was a root). Each changed subsidiary publishes `company.updated` with `changes: [{ field: "parent_company_id", old, new }]` and writes audit. Each subsidiary stores the company it was detached from in `companies.detached_from_company_id` (the module's migration 2). When the company is restored from the trash, the `companies.hierarchy_restore` consumer (event `company.restored`) gives back to it the subsidiaries that are still under the parent they were moved to (those the user has moved elsewhere stay where they are), respecting cycle and depth, with `company.updated` + audit per subsidiary, and clears the mark. Permanent deletion relies on `ON DELETE SET NULL`.
- **Merge** (`modules/duplicates/service/merge.rs`): before writing the winner, its parent is chosen, in this order, among the parent resulting from the merge, the winner's current parent and the removed company's parent, discarding any candidate that is in the subtree of the winner or of the removed company (this handles the case where the winner is a descendant of the removed company: it moves up into the removed company's place). Then, the removed company's direct subsidiaries (except the winner) are pointed to the winner, with `company.updated` + audit per subsidiary. `reparent_children` checks cycle and maximum depth for each subsidiary under the new parent (422 `company_hierarchy_cycle`/`company_hierarchy_too_deep`): deletion only reduces levels, but a merge can add the removed company's subtree under the winner.

## "Belongs to group" filter

The `RecordQuery` `in_group` operator (see [`custom-fields.md`](custom-fields.md#company-groups)): `filter[company_id][in_group]=<id>` on contacts and deals, `filter[id][in_group]=<id>` and `filter[parent_company_id][in_group]=<id>` on companies, and custom `record` → `company` fields. It matches the given company and all its descendants. The web offers the operator in the filter builder for these fields (the field schema lists `in_group` in `operators`) and the lists accept `filter[field][operator]=value` in the URL (converted into the view's filter).

## Tree and totals

- `GET /api/v1/companies/{id}/hierarchy`: walks up to the root and returns the whole group as a flat list, depth-first (siblings by name), with `parent_id`, `depth`, name, owner, archived, number of listed subsidiaries, contacts and open deals (count and sum of `amount_base`) per company. Up to 1000 companies (`truncated`).
- **Companies without read access:** they are omitted, except when they are on the path between the root and a readable company; then they appear as an `accessible: false` node, with no name or numbers, only to keep the shape of the tree. Their id is already visible through the `parent_company_id` of the readable subsidiaries, so nothing new leaks.
- `GET /api/v1/companies/{id}/group-totals`: for the company and all its descendants (not the whole group). Readable companies, contacts (`company_id` in the group), open and won deals (count and sum of `amount_base` in the organization's base currency).
- The numbers come from the `AggregateEngine` with the `in_group` filter, so they respect the read scope of contacts and deals and the field permissions (null sums when `amount_base` is hidden), and leave out archived records. Without `contacts.read`/`deals.read`, the corresponding fields come out null.

## Web

"Group" tab in the company detail (`CompanyGroupTab`): group totals (with shortcuts to the contact and deal lists filtered by `in_group`), a navigable tree with the current company highlighted, and, for those with `companies.update`, the actions to set/change/remove the parent and add a subsidiary (choose an existing company, which then gets this one as its parent). The `company_hierarchy_*` errors become dedicated messages in the `company-hierarchy` namespace.
