# Automatic lead distribution

`assignment` module (`back/modules/assignment`, enabled by default).

Distribution rules choose the owner of a record (`contact`, `company`, `deal` or a custom object with an owner) among salespeople, by round robin, by weight or by least load, with optional routing by field value (region, source…) and respecting away or suspended users. The rules are used by the `assign_by_rule` automation action, by incoming webhooks and by imports. The screen is at **Settings → Lead distribution** (`/settings/assignment`), with a simulation of the next owners.

## Pieces

| Piece | Where it lives | What it does |
| --- | --- | --- |
| Strategy, members, routes, selection (`pick`, `simulate`) | `back/modules/assignment/src/domain.rs` | Pure rules: who is eligible, which route matches and who is next |
| `service::{check, preview, assign, distribute, resolve_pool}` | `back/modules/assignment/src/service/distribution.rs` | Distribution over `&Ctx` (`assign` locks the state in the transaction); used by automation, incoming webhooks, imports and scheduling |
| `service::{list, get, create, update, delete, simulate}` | `back/modules/assignment/src/service/mod.rs` | Rule CRUD (event + audit) and simulation |
| `AssignByRule` | `back/modules/assignment/src/automation.rs` (registered with `app.action` in the module) | The automation engine's `assign_by_rule` action |
| `DistributionStore` | `back/modules/assignment/src/store.rs` | SQL for the rules, the round-robin state (written over `&mut MySqlTx`) and the auxiliary queries (user status, teams, open deals) |
| API | `back/modules/assignment/src/http.rs` (DTOs in `dto.rs`) | `/api/v1/assignment/rules` |
| Screen | `web/src/modules/assignment` (i18n namespace `assignment`) | List, form and simulation |

## Model

- **`assignment_rules`** (module migration `0001_create_assignment.sql`, formerly `1850`–`1853`): stable and unique `key`, `name`, `description`, `record_type`, `strategy`, `is_active`, routing (`routing_field`, `routes` in JSON, `routing_fallback`).
- **`assignment_members`**: rule members in order (`position`), with `weight` (1–100) and absence (`is_away` with an optional `away_from`/`away_until` period).
- **`assignment_states`**: one row per rule with `sequence` (total assignments), last user and date. This is the row locked with `SELECT … FOR UPDATE`.
- **`assignment_counters`**: per (rule, user): `assigned`, `last_sequence` (in which assignment they last received) and `credit` (weighted round robin).
- **`assignment_assignments`**: log of each assignment (user, record when known, source `automation|incoming_webhook|import`) with a unique `idempotency_key`.

## How the next owner is chosen

1. **Pool.** Without routing, the pool is the rule's members. With routing, the value of the record's `routing.field` field (text, number, boolean, lists and option values; compared ignoring case and spaces) is compared with each route's `values`, in order; the first one that matches defines the pool: the route's `user_ids` plus the members of the `team_id` team. With no route, `fallback: members` uses the rule's members and `fallback: none` leaves the record without an owner.
2. **Eligibility.** Users who are not active (invited or deactivated) and members marked as away at the moment (`away` and, if set, within `away_from`–`away_until`) are removed from the pool. Weight and absence apply to the user wherever they appear (members or routes); someone who only appears in a route has weight 1.
3. **Strategy.**
   - `round_robin`: whoever has gone the longest without receiving from the rule gets it (lowest `last_sequence`; ties broken by pool order). Away and new members join the queue naturally.
   - `weighted`: smooth weighted round robin (each pick adds the weight to the credit of each eligible user, picks the highest credit and subtracts the total from the picked one). Weights 3 and 1 give `A, B, A, A` every four.
   - `least_loaded`: fewest open deals (`deals.status = 'open'`, excluding archived/deleted) as owner; ties broken by whoever has gone the longest without receiving.

## Concurrency and idempotency

`assign` opens a transaction, locks the rule's `assignment_states` row (`FOR UPDATE`), re-reads the rule and the counters (also with `FOR UPDATE`), picks, **applies the assignment** and only then writes the counters, state and log and commits. Simultaneous assignments for the same rule queue on the lock, so two never use the same state (validated with 20 concurrent creations: 10/10 with round robin and 12/4 with 3:1 weights). If applying fails, nothing is written and the round robin does not advance.

- **Automation**: applying is the field update through `AutomationGateway::update_record` (same use cases, events and audit as the API) inside the lock window. The idempotency key is `automation:<run_id>:<step_id>`; on a retry of the same step, the already chosen user is reapplied without advancing the round robin, which is why the action is `retry_safe`.
- **Incoming webhook and import**: the owner is chosen (and the round robin advances) before the record is created, which is created with `owner_id` already filled in. If creation fails afterwards, that round-robin turn is consumed. The log has no `record_id`.

## `assign_by_rule` action

```json
{ "id": "distribuir", "type": "action", "action": "assign_by_rule", "config": { "rule_id": "<rule id>", "field": "owner_id" } }
```

- Types: `contact`, `company`, `deal` and `custom.*` with an owner. `field` (default `owner_id`) must be a user field of the type. Required permission: `<type>.update`.
- Validation on save/publish: the rule exists, is active and distributes the same `record_type`.
- `describe` (dry run): returns who would receive it now (`user_id`, `user_name`, `selection`, `route_index`) without advancing the round robin.
- Execution: stores `{ rule_id, field, user_id, user_name, selection, route_index, reused }`. Inactive rule → fails with `assignment_rule_inactive`; no one eligible → fails with `assignment_no_eligible_member`; no route and `fallback: none` → success with `{ user_id: null, skipped: "no_route" }`.
- In the catalog, `rule_id` carries `x-reference: "assignment_rule"` (options come from `GET /assignment/rules?record_type=<type>&active=true`) and `field` carries `x-field-types: ["user"]`.

## Incoming webhook and import

- **Webhook** (`upsert_contact` and `create_deal`): optional `distribution_rule_id` in the action. When the record is **created** without a mapped `owner_id`, the owner comes from the rule (the payload's mapped values feed the routing). Contact updates do not redistribute. When saving the webhook, the rule must exist, be active and be of the same type. If the rule is deleted later, the record is created without an owner from the rule.
- **Import**: `options.distribution_rule_id` in `PUT /imports/{id}/mapping`. Rows created without `owner_id` get the owner from the rule; updated rows (duplicates) do not change.

## API

| Route | What it does |
| --- | --- |
| `GET /assignment/rules` (`record_type`, `active`) | List with members (name, active, away now), routing and usage |
| `POST /assignment/rules` | Creates (`assignment_rule.created`) |
| `GET/PATCH/DELETE /assignment/rules/{id}` | Detail, edit with `If-Match` (`members` replaces the list, `routing: null` removes it) and deletion |
| `POST /assignment/rules/{id}/simulate` `{ count?, values? }` | Next `count` (default 10, up to 50) from the current state, without writing; `values` simulates the routing |

Managing requires `assignment.manage` (administrative, registered by the module with `grant_to(["admin"])`). Listing, viewing and simulating also accept `automations.manage`, to choose the rule in the automation builder. Mutations publish an event and audit with the `assignment_rule` record. Error codes in [`docs/api/errors.md`](../api/errors.md).

## Pending items

- **Configuration package**: the rules are not part of `ConfigSection`. They reference users and teams, which have no stable key and are not portable between installations; a section would only make sense exporting the strategy and the routes without people, which is not installable without a manual user mapping.
- Manual assignment still goes through the record's owner field. There is no screen yet with the assignment history (`assignment_assignments`).
