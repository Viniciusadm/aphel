# `insights` module (extra widgets)

Reference module for **contributed widgets** on the dashboard (`app.widget(..)` with `WidgetProvider`). It has no tables, routes, permissions or events of its own: the data comes from the kernel's aggregation engine and record query engine (`AggregateEngine`, `RecordQuery`), from the `contact` `RecordTypeHandler` and from the scope's field set, always with the scope of the user viewing the dashboard (`own`/`team`/`organization` visibility and hidden fields already applied by the core).

- Backend: `back/modules/insights` (crate `aphel-mod-insights`, struct `InsightsModule`; `deals` and `activities` optional).
- Frontend: `web/src/modules/insights` (renderers, i18n in the `insights` namespace).

## Module settings

| Key | Type | Use |
| --- | --- | --- |
| `birthday_field` | `Field { record_type: "contact", field_types: ["date"] }` | Contact date field used by **This month's birthdays**. Optional; without it the widget shows "not configured" with a link to `/integrations/modules` |

## Widgets

All of them validate their settings in `normalize_settings` (types, limits and **unknown keys** → 422 `validation_error` with `settings.<key>`).

### `insights.deals_by_source` — Deals by source

- Permission: `deals.read`. Uses the dashboard period (field `created_at`) and the person/team filter (`subject_filter`).
- Settings: `limit` (3 to 20, default 8), `only_won` (default `false`, filters `status = won`).
- Aggregation: `group_by` on the system field `source` (text), metrics `count` and `sum:amount`, sorted by count. The remaining sources (when `truncated`) become "Other sources" (total minus the rows shown).
- `amount` hidden from the role: the widget first queries the fields visible to the scope and, if the field is not there (or if the core still responds 422 `field_not_accessible`), redoes it with count only (`amount_visible: false`). `source` hidden: `status: "source_hidden"`.
- Sums ignore each record's currency (same rule as the dashboard); the currency shown is the organization's.

### `insights.team_activity_ranking` — Team activity ranking

- Permission: `activities.read`. Uses the period (field `completed_at`) and the person/team filter.
- Settings: `limit` (1 to 20, default 5), `by_type` (default `false`).
- Aggregation: activities with `status = done` grouped by `owner_id` (the group label is already the user's name), top N by count. With `by_type`, a second aggregation by `owner_id` + `activity_type_id` restricted to the owners in the top N (label = type name).

### `insights.birthdays` — This month's birthdays

- Permission: `contacts.read`. Uses neither the period nor the person filter (organization-wide list within the user's visibility).
- Widget setting: `limit` (5 to 50, default 10).
- "Current month" is computed in the organization's time zone (`request.timezone`).
- States: `not_configured` (no `birthday_field`), `field_unavailable` (field deleted, of another type or hidden from the role) and `ok`.

**Strategy (the AST filter has no "month regardless of year"):**

1. For each 40-year block from 1900 to the current year, an aggregation with an `or` of `between [YYYY-MM-01, YYYY-MM-last]` on the configured field, `group_by` on the field with `day` granularity and `count` (limit of 1000 groups). Result: the existing dates in the month and how many contacts there are on each one (the total shown comes from here).
2. The dates are sorted by day starting from today (days already past go at the end) until they cover `limit` contacts.
3. `find_records` in blocks of 40 dates (`or` of `eq`), sorted by name, up to `limit` ids.
4. `get_record` on each id for name and date (with hidden fields already removed); the list comes out sorted by day and name, with a `/contacts/<id>` link.

Known limits: dates before 1900 are left out; if a 40-year block has more than 1000 distinct dates in the month, the response comes with `truncated: true` and the widget shows a warning. The cost is 4 aggregations + up to `limit/40` searches + up to `limit` reads per load.

## Data format (the widget's `data`)

```json
{ "status": "ok", "currency": "BRL", "only_won": false, "amount_visible": true,
  "rows": [{ "source": "Indicação", "count": 4, "amount": "12000" }],
  "other": { "count": 3, "amount": "800" }, "total": { "count": 7, "amount": "12800" } }

{ "by_type": true, "total": 18,
  "rows": [{ "position": 1, "user_id": "…", "name": "Ana", "count": 9,
             "types": [{ "id": "…", "name": "Ligação", "count": 6 }] }] }

{ "status": "ok", "month": 10, "today": 21, "field_label": "Aniversário", "total": 3, "truncated": false,
  "items": [{ "id": "…", "name": "Bruno Lima", "day": 21, "is_today": true }] }
```

## Frontend

The web module's `register` (`defineModule` in `web/src/modules/insights/index.tsx`) registers the three renderers with `app.widget` (`render`, `isEmpty`, `settings`). The settings labels use namespaced keys (`insights:widgetSettings.*`), resolved by the dashboard's settings form. Horizontal bar charts (Recharts) with `role="img"`, `aria-label` and a readable table/list alongside; colors come from the theme tokens (`--primary`, `--border`, `--muted-foreground`), so they work in the dark theme.
