# P9: second identity

Status: P9.3, P9.4 and P9.5 done. P9.1 and P9.2 wait for the restaurant (decision D5: deferred; no restaurant code exists in this repository).

## P9.3 A plugin written only from the docs

`aphel-loyalty` (its own repository, next to `aphel/`) was made with `aphel-plugin new`, `docs/plugins/README.md`, `host-catalog.md` and the SDK signatures, without reading other plugins' code and without editing any central file.

It declares an entity `loyalty.account`, settings, an event consumer for `deal.won` (durable effect), the contract `loyalty@1` with two operations and one route. Verified on a sales installation: install (dry run, then real), a won deal of 1500 created an account with 1500 points at the silver tier through the event, the route and the operation catalog show it, and `DELETE ?purge=true` removed its entity, records and plugin row while the contact stayed (C8).

Gaps found in the documentation (fixed in `docs/plugins/README.md`):

1. Entity permissions are `<key>.<entity>.read|…`, not `custom.<key>_<entity>.*`.
2. Record types in manifests (`target`) and in `ctx.records()` must be canonical (`people.person`, `deals.deal`); `contact` is refused.
3. A `list` filter addresses an entity's own fields by key (`contact`), not `custom_fields.contact`.

Not exercised: C6 needs two consumers of `loyalty@1`; only the provider side was built.

## P9.4 Core changes required

One. The plugin's own entity permissions were lost for administrators.

| Change | Where | Why |
| --- | --- | --- |
| `PermissionSet::restricted_with(extra, allows)` and `AccessScope::restricted_with` | `kernel/src/domain/access/access_scope.rs` | For a scope that grants everything, restriction started from the static permission catalog only, so the dynamic permissions of the plugin's entities (`loyalty.account.read`) were dropped and every call of the plugin on its own entity failed with `permission_denied` |
| `capability_scope` passes the manifest's entity permissions | `back/modules/wasm_runtime/src/runtime.rs` | Uses the new method; roles with explicit grants were unaffected |

The expected number was zero; the gate "the restaurant needed no core change" must count this one.

## P9.5 A third identity: a small store

Goal: sell products, track stock and take payments, reusing what the restaurant would also use.

| Piece | Provides | Reuses |
| --- | --- | --- |
| `party` | Customers and suppliers as `people.person` / `organizations.organization` | `party.contact_points` (messaging, sequences, privacy work unchanged) |
| `orders` | Entity `orders.order` and item table | `workflow.staged` (board by order status), `line_items.attachable` and `money.amount` (items, totals), automations |
| `inventory` | Entity `inventory.stock_item` and movements; operations `inventory.reserve` and `inventory.release` | `catalog_items` for products; contract `inventory@1` |
| `payments` | Entity `payments.payment`; contract `payments@1` with `payments.charge` and `payments.refund` | `money`, `approvals` for refunds above a limit |
| `delivery` (optional) | Entity `delivery.shipment` | `workflow.staged`, `calendar` |

The `store` blueprint would require the kernel only, recommend `people`, `catalog_items`, `orders`, `inventory`, `payments` and the shared plugins, and ship a configuration package with an order pipeline (new, paid, packed, shipped, delivered, cancelled), order statuses and a low-stock automation.

Open questions: `orders` and `inventory` need a common line-item contract (today `line_items` is deal-shaped), and `payments` needs a provider contract so a gateway can be a WebAssembly plugin.
