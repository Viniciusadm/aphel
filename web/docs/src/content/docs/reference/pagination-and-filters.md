---
title: Pagination and filters
description: How list routes paginate, filter, sort, search and embed relationships, with the operators, value formats and validation rules of the Aphel query language.
sidebar:
  order: 4
---

Every record listing in Aphel (`/people`, `/organizations`, `/deals`, `/tasks`, `/objects/{key}/records`, `/entities/{entity}/records` and so on) uses the same query language. It is parsed once, validated against the fields of the record type and the field permissions of the caller, and never turned into SQL from user text.

## Pagination

| Parameter | Type | Default | Limits |
| --- | --- | --- | --- |
| `page` | integer | `1` | 1 or more |
| `per_page` | integer | `25` | 1 to 100 |

The response carries the total number of matching records:

```json
{
  "data": [
    { "id": "0192a1b2-0000-7000-8000-000000000001", "name": "Ana Souza" }
  ],
  "meta": { "total": 120, "page": 1, "per_page": 25 }
}
```

`meta.total` counts all records that match the filter, not only the current page. A page past the end returns an empty `data` array. A value that is not an integer within the limits answers `422 validation_error` with the invalid parameter in `details.fields`:

```json
{
  "error": {
    "code": "validation_error",
    "message": "invalid pagination parameters",
    "details": {
      "fields": [{ "field": "per_page", "message": "must be an integer between 1 and 100" }]
    }
  }
}
```

A few listings use cursors instead of pages (for example the record timeline). They return `meta.next_cursor` and accept the cursor back in a `cursor` parameter; their routes say so in the OpenAPI document.

## Parameters of a record listing

| Parameter | Purpose |
| --- | --- |
| `filter` | A filter tree encoded as JSON. |
| `filter[field]=value` and `filter[field][operator]=value` | Simple conditions in the query string. |
| `sort` | Comma-separated sort fields. |
| `search` | Free-text search. Many listings also accept `q` as an alias. |
| `archived` | `exclude` (default), `include` or `only`. |
| `include` | Comma-separated relationships to embed. |
| `page`, `per_page` | Pagination. |

## Simple filters

Each `filter[...]` parameter is one condition. The parts are the field key and, optionally, the operator; without an operator the condition is `eq`. All simple conditions are combined with `and`, and at most 50 of them are accepted in one request.

```bash
curl -G "$APHEL_URL/api/v1/people" \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  --data-urlencode "filter[email][contains]=@acme" \
  --data-urlencode "filter[owner_id][in]=me" \
  --data-urlencode "filter[created_at][gte]=2026-01-01" \
  --data-urlencode "sort=-created_at,name" \
  --data-urlencode "per_page=50"
```

How the value is read depends on the operator:

| Operator kind | Operators | Value in the query string |
| --- | --- | --- |
| None | `is_empty`, `is_not_empty`, `today`, `yesterday`, `tomorrow`, `this_week`, `last_week`, `next_week`, `this_month`, `last_month`, `next_month`, `this_year` | Ignored. Send an empty value or any text: `filter[phone][is_empty]=`. |
| Single | `eq`, `neq`, `contains`, `not_contains`, `starts_with`, `gt`, `gte`, `lt`, `lte`, `in_group`, `in_saved_filter` | The text of the value. |
| Pair | `between` | Two values separated by a comma: `filter[amount][between]=1000,5000`. |
| List | `in`, `not_in` | Comma-separated values, 1 to 100 of them: `filter[owner_id][in]=id1,id2`. Spaces around items are trimmed. |
| Days | `within_last_days`, `within_next_days`, `older_than_days`, `anniversary` | A whole number of days from 0 to 3650. |

Values that contain commas cannot be expressed in the list and pair forms of the simple syntax; use the JSON form.

## JSON filters

The `filter` parameter takes a JSON document and supports `or`, nested groups and conditions on related records. Remember to URL-encode it.

```bash
curl -G "$APHEL_URL/api/v1/people" \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  --data-urlencode 'filter={"op":"or","conditions":[{"field":"email","operator":"contains","value":"@acme"},{"field":"city","operator":"eq","value":"Recife"}]}'
```

A filter node is one of three shapes:

| Node | Shape |
| --- | --- |
| Group | `{ "op": "and" or "or", "conditions": [ node, ... ] }` |
| Condition | `{ "field": "...", "operator": "...", "value": ... }`. Omit `value` for operators of kind none. |
| Related | `{ "related": { "relationship": "company", "filter": node, "none": false } }`. Matches records that have a linked record satisfying the filter, or, with `"none": true`, records that have none. |

Values are JSON: a string for single operators, an array for `in` and `not_in`, an array of exactly two items for `between`, and a string or number of days for the day operators. Objects reject unknown properties, so a typo answers `422`.

Limits of a filter:

| Limit | Value |
| --- | --- |
| Nesting depth of groups | 4 |
| Conditions in one request | 50 |
| Values in an `in` or `not_in` list | 100 |
| Days for the day operators | 0 to 3650 |

When both forms are sent, the JSON filter and the simple conditions are combined with `and`.

## Operators

| Operator | Meaning |
| --- | --- |
| `eq`, `neq` | Equal and not equal. |
| `contains`, `not_contains` | Text contains or does not contain the value as a substring. Matching follows the collation of the database column. |
| `starts_with` | Text starts with the value. |
| `gt`, `gte`, `lt`, `lte` | Greater than, at least, less than, at most. |
| `between` | Within an inclusive range of two values. |
| `in`, `not_in` | Equals any of the listed values, or none of them. |
| `is_empty`, `is_not_empty` | The field has no value, or has one. |
| `today`, `yesterday`, `tomorrow` | The date falls on that day in the time zone of the organization. |
| `this_week`, `last_week`, `next_week` | The date falls in the week that starts on Monday. |
| `this_month`, `last_month`, `next_month`, `this_year` | The date falls in that calendar month or year. |
| `within_last_days` | Today and the previous N days. |
| `within_next_days` | Today and the next N days. |
| `older_than_days` | Before the day N days ago. |
| `anniversary` | The month and day recur within the next N days (birthdays and similar). |
| `in_group` | The referenced record is the given record or one of its descendants in a hierarchy, available on reference fields that point to hierarchical record types (for example company groups) and on `id`. |
| `in_saved_filter` | The record matches a stored segment, with the field `segment` and the segment id as the value. Available when the `segments` module is enabled and only on routes that accept stored filters. |

### Operators by field type

A field accepts only the operators that make sense for its type; others answer `422` on `...operator`.

| Field type | Operators |
| --- | --- |
| `text`, `url`, `email`, `phone` | `eq`, `neq`, `contains`, `not_contains`, `starts_with`, `in`, `not_in`, `is_empty`, `is_not_empty` |
| `long_text` | `contains`, `not_contains`, `is_empty`, `is_not_empty` |
| `number`, `currency` | `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `between`, `in`, `not_in`, `is_empty`, `is_not_empty` |
| `date`, `datetime` | `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `between`, every relative-date operator and `is_empty`, `is_not_empty` |
| `select`, `user`, `record` | `eq`, `neq`, `in`, `not_in`, `is_empty`, `is_not_empty` (plus `in_group` for hierarchical references) |
| `multi_select` | `in`, `not_in`, `is_empty`, `is_not_empty` |
| `checkbox` | `eq`, `neq` |
| `file`, `image`, `document` | `is_empty`, `is_not_empty` |

Custom field types registered by modules declare their own operators. The exact operators and value kinds of each field of a record type are returned by `GET /api/v1/fields/{record_type}/schema` and, for custom objects, by `GET /api/v1/objects/{key}/schema`.

## Value formats

| Field type | Format |
| --- | --- |
| `text`, `long_text`, `url`, `email` | Non-empty string. |
| `phone` | A phone number; it is normalized to E.164 using the country of the field before comparing. |
| `number`, `currency` | A decimal number, as a JSON number or a string. |
| `date` | `YYYY-MM-DD`. |
| `datetime` | RFC 3339, such as `2026-10-20T13:45:00Z`, or just `YYYY-MM-DD`: a date alone covers the whole day in the time zone of the organization. For `between`, both bounds must be dates or both must be date-times. |
| `checkbox` | `true` or `false`. |
| `select`, `multi_select` | The option `value`, not the label. An option that does not exist answers `422`. |
| `user` | A user id, or `me` for the caller. |
| `record` | The id of the referenced record. |

## Special fields

Besides the fields of the record type, filters accept:

| Field | Operators | Meaning |
| --- | --- | --- |
| `id` | `eq`, `neq`, `in`, `not_in`, `in_group` | The record id. |
| `owner_team` | `in`, `not_in`, `is_empty`, `is_not_empty` | Team ids of the record owner. An owner in two teams matches both. Only on record types with owners. |
| `tags` | `in`, `not_in`, `is_empty`, `is_not_empty` | Tag ids, when the `tagging` module is enabled. |
| `<relationship>.<field>` | The operators of that field | A field of a single related record, such as `company.industry`. |

Related conditions only see records that the caller can read.

## Sorting

`sort` is a comma-separated list of field keys. A leading `-` sorts descending and a leading `+` or nothing ascending. At most 3 sort fields are allowed, and `id` is always accepted.

```text
sort=-created_at,name
```

Without `sort`, most listings return the newest records first. A field that cannot be sorted answers `422` on `sort.<index>.field` with `cannot be used to sort`.

## Search

`search` runs a substring match over the searchable text fields of the record type. The documentation of each route lists which fields it covers (for people: name, email, phone and mobile). Terms have at most 100 characters; phone-like terms are normalized for phone fields. Many listings accept `q` as an alias of `search`, and the explicit `search` wins when both are present. For search across every record type, use the global search of the `search` module.

## Archived records

| Value | Result |
| --- | --- |
| `exclude` | Only active records. This is the default. |
| `include` | Active and archived records. |
| `only` | Only archived records. |

A record type that cannot be archived answers `422` on `archived` for `only`.

## Embedding relationships with include

`include` takes a comma-separated list of relationship names to embed in each record, such as `include=company,owner`. Every route that supports it lists the allowed names in its OpenAPI description, and enabled modules can contribute more. An unknown name answers `422`:

```json
{
  "error": {
    "code": "validation_error",
    "message": "invalid include",
    "details": {
      "fields": [{ "field": "include", "message": "friend is not available; allowed: company, owner" }]
    }
  }
}
```

## How filters are validated

Validation collects every problem of the request and answers a single `422` with one entry per problem. The `field` of each entry is a path into the request:

| Path | Example problem |
| --- | --- |
| `filter` | `a filter can have at most 50 conditions`, `must be a filter group or condition encoded as JSON` |
| `filter.conditions.0.field` | `is not a known field`, `cannot be used in filters`, `is not accessible` |
| `filter.conditions.0.operator` | `starts_with is not supported by number fields` |
| `filter.conditions.0.value` | `must be a list of values`, `must be a number`, `option "x" does not exist`, `must be omitted for is_empty` |
| `filter[email][nope]` | `unknown filter operator` (simple form; the field is the whole key) |
| `sort`, `sort.0.field` | `at most 3 sort fields are allowed`, `is not a known field` |
| `search` | `must have at most 100 characters` |
| `archived` | `must be exclude, include or only` |

### Field permissions

Fields that the role of the caller cannot see are treated as hidden. A filter, sort or search that uses one answers:

```json
{
  "error": {
    "code": "field_not_accessible",
    "message": "the role of the user cannot see these fields",
    "details": {
      "fields": [{ "field": "filter.conditions.0.field", "message": "is not accessible" }]
    }
  }
}
```

The same code appears when a write touches a hidden field, and `field_read_only` when it touches a read-only one. Fields hidden from the role also disappear from responses and schemas. If a request is invalid for other reasons the answer is `422 validation_error`. See the [error table](/_aphel/docs/reference/errors/) for both codes and [Roles and permissions](/_aphel/docs/admin/roles-and-permissions/) for how field permissions are configured.
