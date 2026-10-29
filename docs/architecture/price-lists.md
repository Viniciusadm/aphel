# Price lists, bundles and discount limits

Session 25, track B. Three pieces that the quote (track A) consumes through the contracts in `modules/price_lists/hooks.rs` and `modules/price_lists/bundles.rs`.

## Model

| Table | What it stores |
| --- | --- |
| `price_lists` | Price list: stable, unique `key`, name, description, `currency` (enabled in the organization, `organization_currencies`), `applies_to` (`all`, `segment`, `companies`), `segment_id` (company segment), `valid_from`/`valid_to` (optional, inclusive), `is_active`, `is_default` (at most one default per currency, guaranteed by the generated column `default_marker`) |
| `price_list_companies` | Companies of a list with `applies_to = companies` |
| `price_list_items` | A product's price in the list: `unit_price` and `min_quantity` (tier; unique per product) |
| `product_bundles` | The product is a bundle: `pricing` (`fixed` or `sum`) |
| `product_bundle_components` | Bundle components with quantity and position |
| `role_discount_limits` | Maximum discount (%) per role; no row = no limit |

## Choosing the list (`PriceListResolver`)

`MySqlPriceListResolver::new(pool)` delegates to `PriceListService` (`modules/price_lists/hooks.rs`).

- `default_for(currency, company_id, on)` only considers lists that are active, in the currency and valid on `on`, in this order:
  1. a `companies` list that includes the company;
  2. a `segment` list whose segment contains the company;
  3. the currency's default list.

  Within the same level, the one with the most recent `valid_from` wins (a dated list beats an undated one) and, on a tie, the most recently created one. Segment membership is evaluated by the `RecordQueryEngine` (`in_segment` + `id`) with an organization-wide full read scope (`MySqlSegmentMembership`): the list applies according to the organization's configuration, not according to the visibility of whoever is quoting. An invalid segment filter counts as "not a member".
- `price(price_list_id, product_id, quantity, on)`: `None` when the list does not exist, is inactive or outside its validity period, or does not have the product (the quote falls back to the catalog price). Tiers: the one with the highest `min_quantity` that does not exceed the quantity applies; below all of them, the lowest tier.
- `find` returns the list even if inactive or expired (the quote shows the list it used).

## Bundles (`BundleExpander`)

`MySqlBundleExpander::new(pool)` delegates to `BundleExpansionService` (`modules/price_lists/bundles.rs`). `expand(product_id, price_list_id)` returns `None` if the product is not a bundle. With a list, each price comes from it (valid on today's date in the organization's time zone), falling back to the catalog price; the expansion's currency is the list's, otherwise the bundle's.

- `fixed`: price of the bundle product itself (or from the list).
- `sum`: sum of component price × quantity, rounded to cents.

Rules on save (`PUT /products/{id}/bundle`, `BundleUseCases::save`): 1 to 50 components, no repeats and not the product itself; **a single level only** — a component cannot be a bundle and a product that is a component of another bundle cannot become a bundle (409 `product_bundle_nested`), which also prevents cycles; with `sum`, all components in the bundle's currency. A deleted component disappears from the expansion.

## Discount limit (`DiscountLimits`)

`max_discount_percent(scope, user_id)` returns the limit of the user's role (`users.role_id` → `role_discount_limits`), `None` = no limit. The quote saves `exceeds_discount_limit`, and the default approval policy `quote_discount_limit` (seeded by `seed_quote_discount_policy` when the organization is created and by migration `2535` for existing ones: moment `quote_send`, filter `{"field": "exceeds_discount_limit", "operator": "eq", "value": true}`, one `owner_team_manager` step with rule `any`, position 0) requires approval before sending.

## API and permissions

| Route | Permission |
| --- | --- |
| `GET /price-lists`, `GET /price-lists/{id}`, `GET /price-lists/{id}/items`, `GET /products/{id}/bundle` | `products.read` with organization scope |
| `POST/PATCH/DELETE /price-lists…`, `PUT /price-lists/{id}/items`, `PUT/DELETE /products/{id}/bundle` | `products.update` with organization scope |
| `GET/PUT /discount-limits` | `settings.manage` |

Price lists are a commercial catalog (like products), so they use the product permissions; discount limits affect approval policy, hence `settings.manage`.

Events (with audit in the same transaction): `price_list.created|updated|deleted`, `price_list.items_replaced` (payload with counts; audit with the removed/added tiers), `product_bundle.updated|removed` (record = product), `discount_limits.updated`. By decision, none of them is in the public catalog: they are configuration events with no resource in the data API (the webhook renderer delivers the record in the `GET /api/v1/<resource>/{id}` format, and price lists, bundles and limits are not records), so they stay internal (audit, timeline and Aphel's consumers); commercial catalog changes are carried by configuration packages. The list is in `docs/api/events.md`.

## Configuration packages

Sections `price_lists` (only lists with `applies_to = all`, items by product code) and `discount_limits` (role by key). See [`docs/config-packages/README.md`](../config-packages/README.md).

## Web

`web/src/features/products/price-lists` (list and editing at `/products/price-lists`, with a discount limits tab) and `web/src/features/products/bundles` (bundle section on the product detail). Texts in the `price-lists` namespace.
