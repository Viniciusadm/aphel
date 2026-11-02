# Deal team and credit split

Each deal can have a team in addition to the owner (`deals.owner_id`): users with a configurable role and a percentage of the revenue credit. Code in `back/modules/deals/src/team` (use cases, ports, package section and credit calculation), `back/modules/deals/src/team` (MySQL) and `web/src/features/deals/team`.

## Data

- `deal_member_roles`: configurable roles (stable `key`, `name`, `position`, `is_active`). `sales`, `pre_sales`, `specialist` and `manager` are seeded on installation (`seed_deal_member_roles`, called by `seed_defaults`). CRUD at `/api/v1/deal-member-roles` with `settings.manage` (listing requires `deals.read`). Deleting a role leaves its members without a role (`ON DELETE SET NULL`); deactivating it prevents choosing it again.
- `deal_members`: `deal_id`, `user_id` (unique per deal), `role_id`, `credit_percent DECIMAL(5,2)`, `added_by`. Removed by cascade when the deal is permanently deleted (the trash keeps the team).

## Credit rule

- The members' sum stays between 0 and 100%; above that the API responds 422 `deal_credit_exceeds_total` (with `allocated_percent` and `available_percent` in `details`). The check runs with the deal row locked (`SELECT … FOR UPDATE`), so two simultaneous additions do not exceed 100%.
- The owner implicitly receives `100 − sum` (it does not become a row). If the owner is also a member, their credit is the row plus the remainder. Without an owner, the remainder is credited to no one.
- Changing the owner does not touch the team; the new owner starts receiving the remainder.
- A user removed from the organization stays on the team (history and credit remain; `user_active = false` in the response) and loses access for no longer having a scope. They can be removed from the team manually.

## Access

The `deal` type declares `member_access("deal_members", "deal_id", "user_id")` in the `RecordTypeRegistry`. With an `own` (or `team`) read scope, the user also sees the deals they are a member of, and nothing else. This applies to everything that goes through `push_record_scope_as` (lists, detail, search, timeline, attachments, notes, dashboard, forecast, export). Editing, moving, winning/losing, archiving and deleting still follow the owner rule: the filter only includes membership for `.read`/`.export` permissions. The pipeline restriction by team still applies. Details in [`custom-fields.md`](custom-fields.md#from-filter-to-sql).

Changing the team requires `deals.update` on the deal (the same rule as editing it); reading requires reading the deal.

## Events

`deal.member_added`, `deal.member_updated` and `deal.member_removed`, with the deal record, `member_id`, `user_id`, `user_name`, `role_*`, `credit_percent` and `changes` (`member.user_id`, `member.role`, `member.credit_percent`), plus audit in the same transaction. They are public (webhooks) and appear in the deal's timeline. Roles publish `deal_member_role.created|updated|deleted|reordered`.

## Goals and forecast

- Goals have a `credit_mode`: `owner` (default, full amount to the owner) or `split`. With `split`, user or team `won_amount` goals add up, for each deal won in the period, `amount × share`, where the share is the user's credit as a member plus, when they are the owner, the remainder. For a team, it adds up the share of all its members. Organization goals and count goals do not change.
- The forecast filtered by `owner_id` or `team_id` (one of the two) always uses the split: it includes the deals the user is a member of and weights the amounts by the share. Without members, the result is the same as in owner mode. The forecast's deal list shows the full amount of each one.
- The summed amount is the same field as the metric (`amount_base`, base currency). The query lives in `MySqlDealCreditReader` (`push_credit_share`/`push_credited_filter`) over the resolved `QueryPlan`, with the read visibility of whoever is querying.

## Configuration packages

Section `deal_member_roles` (`DealMemberRolesSection`): exports the active roles by key; installing creates or updates (and reactivates) and never deletes or touches the members.
