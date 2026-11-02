# Aggregation and reporting engine

Reports, goals, forecast and dashboard widgets compute numbers through the same engine: `AggregateEngine` (`back/kernel/src/application/query/aggregate`). It extends the session 04 query engine (`RecordQueryEngine`) and keeps the same guarantees:

- **Read scope:** the use case calls the type's `scope.record_filter(&read_permission)`; the result (`own`, `team`, `organization`) goes into the `WHERE` exactly as in lists. Related records used in filters, groupings or labels also go through the related type's scope.
- **Field permissions:** fields come from `FieldSet::for_scope`. A field hidden from the role cannot be grouped, aggregated, filtered or used as the period (422 `field_not_accessible`).
- **No SQL with user text:** the query is validated and resolved into an `AggregatePlan` (groupings, metrics, sorting and the filter's `QueryPlan`). Tables and columns come from the `RecordTypeRegistry`; values go in as parameters.

## Input

```json
{
  "record_type": "deal",
  "filter": { "op": "and", "conditions": [{ "field": "status", "operator": "eq", "value": "won" }] },
  "group_by": [{ "field": "closed_at", "granularity": "month" }, { "field": "owner_id" }],
  "metrics": [{ "fn": "count" }, { "fn": "sum", "field": "amount" }],
  "period": { "field": "closed_at", "preset": "quarter" },
  "compare_to_previous": true,
  "sort": [{ "key": "sum:amount", "direction": "desc" }],
  "limit": 500
}
```

In Rust: `AggregateQuery::new("deal").group(GroupBySpec::by_time("closed_at", TimeGranularity::Month)).metrics(vec![MetricSpec::count(), MetricSpec::of(AggregateFunction::Sum, "amount")]).period(AggregatePeriod::preset("closed_at", "quarter")).compare(true)` and `AggregateEngine::new(&ports).run(&scope, query)`. Through the API: `POST /api/v1/aggregates`; what can be grouped/aggregated is returned by `GET /api/v1/aggregates/{record_type}/schema`. On the web: `useAggregate`/`useAggregateSchema` in `web/src/features/records/aggregates.ts` (the engine's client in the web kernel).

### Groupings (`group_by`, up to 3)

| Field | Group key | Label |
| --- | --- | --- |
| Text, email, URL, phone, number, currency | the value (normalized number) | — |
| System select | the option's `value` | option label |
| Custom select/multi-select | option id | option label |
| User (`owner_id`, `user` fields) | user id | user name |
| Record (`company_id`, `stage_id`, `pipeline_id`, `lost_reason_id`…) | record id | name/title, only if the user can read the record |
| Checkbox | `"true"`/`"false"` | — |
| Date and date-time + `granularity` | first day of the interval (`YYYY-MM-DD`) | — |
| `tags` (virtual) | tag id | tag name |
| `owner_team` (virtual) | id of the owner's team | team name |
| `relation.field` (e.g. `company.industry` on deals) | like the related type's field | same |

- Granularities: `day`, `week` (starts on Monday), `month`, `quarter`, `year`. Without a granularity, date fields group by day. Long text cannot be grouped.
- **Time zone:** dates/times are stored in UTC; the engine converts them to the organization's time zone before building the interval. The offset is computed in Rust (`timezone_shift`, including daylight saving time changes within the window) and sent as parameters (`DATE_ADD(col, INTERVAL CASE ... END SECOND)`), without relying on MySQL's time zone tables.
- Empty values form the `null` group (always last in the ordering).
- Groups by a select field (system or custom, including multi-select) are ordered by the **option position** configured on the field, not alphabetically by value: the `ORDER BY` uses `FIELD(g, ?, ?, …)` with the option keys in `position` order (`GroupLabels::Options` keeps the ordered list). Values that are no longer options of the field come after the options, and `sort` with `direction: desc` reverses the option order.

### Metrics (`metrics`, up to 10)

- `count` without a field: distinct records (`COUNT(DISTINCT r.id)`). With a field: records with a value in that field.
- `sum`, `avg`, `min`, `max`: number or currency fields of the type itself. Sums ignore each record's currency (same rule as the dashboard).
- **Duration between two dates:** with `to` (a second `date`/`datetime` field of the type itself, different from `field`) and `unit` (`days`, the default, or `hours`), the metric applies `fn` (`count`, `sum`, `avg`, `min`, `max`) to the time from `field` to `to` of each record: `TIMESTAMPDIFF(SECOND, field, to) / 86400` (or `/ 3600`), with fractions. Records missing either date are left out (`count` counts only those that have both). E.g. average time to close a deal, `{ "fn": "avg", "field": "created_at", "to": "closed_at", "unit": "days" }`. Both fields go through the same validation as any metric (exist, be active, not be hidden from the role, belong to the type itself) and can be system, computed or custom fields. In Rust: `MetricSpec::duration(AggregateFunction::Avg, "created_at", "closed_at", DurationUnit::Days)`. The metric's column in the result carries `field_type: number`, `to`, `to_label` and `unit`.
- The metric key is `fn` or `fn:field` (`count`, `sum:amount`), or `fn:field..to:unit` for durations (`avg:created_at..closed_at:days`), used in `sort` and to read the result.

### Period and comparison

- `period.field` is a date field of the type itself; `preset` accepts `today`, `yesterday`, `last_7_days`, `last_30_days`, `week`, `last_week`, `month`, `last_month`, `quarter`, `last_quarter`, `year`, `last_year` or `custom` with `from`/`to` (up to 731 days). The boundaries are whole days in the organization's time zone.
- `compare_to_previous` computes the same report over the previous period (the domain's `previous_period`): intervals of whole months go back the same number of months (March → February, quarter → previous quarter); any other interval goes back its own length in days. Each row and the totals carry `previous`.
- When a grouping is the period field itself with a granularity, the rows of the previous period are matched interval by interval (1st month with 1st month). When grouping only by that field, empty intervals show up with zero.

## Relationships (cross reports)

Each type declares its relations in the `RecordTypeRegistry` (`RecordRelation` with one or more `RelationPath`):

| Type | Relation | Path |
| --- | --- | --- |
| Deal | `company`, `contact` | `BelongsTo` (`company_id`, `contact_id`) |
| Deal, contact, company | `activities` | `PolymorphicMany` (`activities.record_type/record_id`) or `Linked` (`activity_links`) |
| Contact | `company` | `BelongsTo` (`company_id`) |
| Contact | `deals` | `HasMany` (`deals.contact_id`) |
| Company | `contacts`, `deals` | `HasMany` (`company_id`) |
| Activity | `contact`, `company`, `deal` | `Polymorphic` (main record) or `Linked` (`activity_links`) |
| Any type connected by a configurable relationship (session 16) | `key` on the `from` type, `inverse_key` on the `to` type | `Relationship` (`record_relations`), single-record when the cardinality allows it (`to` in `one_to_many`, both in `one_to_one`) |

Rules to avoid duplicating totals:

1. **Filters on related records** (`company.industry eq "Varejo"`, `deals.status eq "won"` on companies) become `EXISTS (...)` — never `JOIN` — and mean "there is at least one related record, visible to the user and not deleted, that meets the condition". Each condition is evaluated independently: two conditions on `deals` inside an `and` can be met by different deals. Archived related records count; deleted ones do not.
2. **Grouping by a related field** is only allowed on single-record relations (`BelongsTo`, like `company.*` on deals, or a single-record `Relationship`, like `company.*` on contracts of a custom object). The `LEFT JOIN` brings at most one row per record; an invisible or deleted related record falls into the empty group.
3. **Metrics** use only fields of the type itself and count distinct records.
4. Multi-valued groupings (`tags`, multi-select, `owner_team` with an owner in two teams) put the same record in more than one group, on purpose. That is why **totals** are computed in a separate query, without grouping, and never by summing the rows.

## Output

`AggregateResult` carries the columns (`groups`, `metrics`), the rows (`groups[].value`/`label`, `values`, `previous`), `totals` (`values`, `previous`), the resolved `period` and `previous_period`, the time zone used and `truncated` (there are more groups than `limit`, default 500 and maximum 1000). In the API, numbers are returned as decimal strings.

## Where to extend

- New record type: declare it in the `RecordTypeRegistry` (table, owner, permission, fields and relations) and the engine starts aggregating it; record labels come from `RECORD_TITLES` in `kernel/infrastructure/query/aggregate.rs`.
- Dashboard widgets: `DashboardPorts::Query` implements `AggregatePorts`, so a `WidgetProvider` uses `AggregateEngine::new(context.ports.query())`.
- Background execution (scheduled reports): build the recipient's `AccessScope` (`load_access` + `build_access_scope`) and run the engine with it; each recipient only sees their own data.

## Who uses the engine

- **Built-in reports** (`reports` module, `back/modules/reports/src/service/builtin.rs`, SQL in `store/builtin.rs`): conversion and performance. Count and sum metrics come from the engine; time per stage and average cycle come from dedicated queries that join with `deals` and apply `push_where` over a `QueryPlan` resolved with the user's scope and field permissions.
  - `deal_stage_history` stores one row per stage entry (initial row with the deal id; transitions with the id of the `deal.stage_changed` event, which makes the write idempotent). The stage exit is computed in the query with `LEAD()`. The `reports.deal_stage_history` consumer writes new entries; migration `1411` backfills from `domain_events`.
- **Saved reports** (`reports` module, `back/modules/reports/src/service/{reports,schedules,delivery}.rs`): the definition is the `AggregateQuery` JSON. Schedules run the engine with each recipient's `AccessScope` (`load_access` + `build_access_scope`), never with the creator's. The email carries the table (up to 50 rows) and the full XLSX spreadsheet as an attachment (`encode_report` + `Mailer::send_with_attachments`); above `MAX_ATTACHMENT_BYTES` (5 MB) the attachment is omitted and the email points to the report's authenticated link. Email attachments in the kernel: `EmailAttachment { filename, content_type, bytes }` (`kernel/application/mail/ports.rs`), assembled as `multipart/mixed` by the `SmtpMailer`. Idempotent delivery per report/schedule + recipient + period (`report_deliveries` with unique key `schedule_id, user_id, scheduled_for`): before SMTP, the delivery is claimed with status `sending` (row locked with `FOR UPDATE`); an already recorded `sent` or `sending` makes the next attempt end as `AlreadySent` without resending. If SMTP fails on an attempt that still has retries, the `sending` mark is deleted; on the last one it becomes `failed`. If sending succeeds but writing `sent` fails, the row stays `sending` and the email is not repeated (at most once). The `If-Match` of reports and schedules is rechecked inside the transaction with `lock_version` (`FOR UPDATE` on the row) → 409 `version_conflict`. The `saved_report` widget shows up for anyone who can read some record type (including `custom.*` custom objects); execution checks read access by the report's `record_type`.
- **Goals** (`goals` module, `back/modules/goals`): progress is an aggregation with `owner_id in [..]`, `owner_team in [..]` or no owner, over the goal's period, always with the viewer's scope. The `goals_progress` widget returns the current goals with progress already computed on the backend (`items`, `total`). Deleting the user or team that owns a goal nulls the reference (`ON DELETE SET NULL`); the orphan goal disappears from lists and the `goals.purge_orphans` job (hourly) deletes it, publishing `goal.deleted` with `reason = owner_removed` and an audit entry.
- **Forecast** (`forecast` module, `back/modules/forecast`): weighted amount per `expected_close_date` month in dedicated SQL over the same `QueryPlan`; the month's won amount comes from the engine. The forecast configuration is portable through the `forecast` section of configuration packages.
- **Dashboard** (`dashboard` module, `back/modules/dashboard`; web `web/src/modules/dashboard`): the widget contract stays in the kernel (`kernel/application/dashboard/{provider,ports,subject,period}.rs`, `kernel/domain/dashboard`, `MySqlDashboardPorts` in `kernel/infrastructure/repositories/mysql_dashboard.rs`; `period_totals`, `PeriodQuery`/`parse_period`); the module has the built-in widgets, the per-user layout and the organization default (`LayoutStore`), the `/dashboard/*` routes and the `dashboard` section of configuration packages. On the web, the widget contract (`WidgetRenderer`, `SettingsField`) and the `StatTile`/`Delta`/`RecordLinkText` blocks live in `web/src/kernel/widgets`. The `reports` module depends on `dashboard` (period DTOs and `PeriodSelector`/`CompareToggle` on the web). The person/team filter becomes `owner_id eq`/`owner_team in` in the `Planner` and in `context.with_subject(..)`; "compare with the previous period" uses `previous_period`. The `saved_report` widget runs the saved report on the backend (with the dashboard's period when the report has a period field) and returns `status` (`ok`, `no_report`, `unavailable`), `report` and `result`. In the `dashboard` section of packages, `settings.report_id` is not portable: export writes `null` and validation rejects an id (`not_portable`). In `pending_center`, `due_at` and `priority` are returned as `null` when the field is hidden from the role. `recent_activity` applies the `RecordQueryFilter` restrictions (e.g. pipeline per team) to each type read and omits events recorded during an import by the user who started it (`RecentEventExclusion` contribution from the `dashboard` module, active with the `imports` module), so that an import does not flood the widget. Kanban, the deals list and saved reports expand `in_segment` (`SegmentResolution::Expand`).
