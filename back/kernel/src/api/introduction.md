REST API of Aphel CRM. Everything the interface does goes through here: contacts, companies, deals, activities, custom fields, imports and configuration.

## Authentication

Create a token in **Settings → API tokens**, choosing the scopes (`contacts:read`, `deals:write`…) and the expiration. The token is shown only once; send it on every request:

```bash
curl -H "Authorization: Bearer aphel_3f9a1c2b_…" https://your-aphel.example.com/api/v1/contacts
```

The token acts on behalf of whoever created it and can never do more than that person can **at the time of the request**: the effective permission is the intersection of the scopes with that person's current permissions. A revoked or expired token, or one belonging to a deactivated user, responds `401 invalid_api_token`; insufficient scope, `403 insufficient_scope`. Session and account routes (`/auth/*`, `/api-tokens`) only accept the browser session.

## Limits

Each token has a sliding window (default: 600 requests per minute). Every response carries `X-RateLimit-Limit`, `X-RateLimit-Remaining` and `X-RateLimit-Reset` (seconds). When exceeded, `429 rate_limited` with `Retry-After`.

## Responses and pagination

- Item: `{ "data": { ... } }`.
- List: `{ "data": [ ... ], "meta": { "total": 120, "page": 1, "per_page": 25 } }`, with `?page=` (≥ 1) and `?per_page=` (1–100).
- Cursor lists (timeline): `{ "data": [ ... ], "meta": { "next_cursor": "..." } }`.
- Dates in ISO-8601 UTC (`2026-10-20T13:45:00Z`), money as a decimal string with currency (`"amount": "15000.00", "currency": "BRL"`), custom fields in `custom_fields`.

## Filters, sorting and relationships

```bash
# simple form: conditions combined with "and"
curl -G -H "Authorization: Bearer $APHEL_TOKEN" "$APHEL_URL/api/v1/deals" \
  --data-urlencode "filter[status][eq]=open" \
  --data-urlencode "filter[amount][gte]=1000" \
  --data-urlencode "sort=-amount,title"

# full form: the filter tree as JSON
curl -G -H "Authorization: Bearer $APHEL_TOKEN" "$APHEL_URL/api/v1/contacts" \
  --data-urlencode 'filter={"op":"or","conditions":[{"field":"email","operator":"contains","value":"@acme"},{"field":"city","operator":"eq","value":"Recife"}]}'
```

`include=company,owner` embeds allowed relationships (listed on each route).

## Safe writes

- `PATCH` is partial; send `If-Match: <updated_at>` to avoid overwriting someone else's change (`409 version_conflict`).
- Send `Idempotency-Key: <uuid>` on `POST` so you can repeat the request without duplicating the record: the same key with the same body returns the original response (`Idempotent-Replayed: true`).

## Errors

Always `{ "error": { "code", "message", "details" } }`. Decide based on the `code`, which is stable; `details.fields` points to invalid fields in `422 validation_error`.

## Complete example

```bash
export APHEL_URL=https://your-aphel.example.com APHEL_TOKEN=aphel_…
company=$(curl -s -X POST -H "Authorization: Bearer $APHEL_TOKEN" -H "Content-Type: application/json" \
  -d '{"name":"Acme Ltda","custom_fields":{}}' "$APHEL_URL/api/v1/companies" | jq -r .data.id)
contact=$(curl -s -X POST -H "Authorization: Bearer $APHEL_TOKEN" -H "Content-Type: application/json" \
  -d "{\"first_name\":\"Bruna\",\"company_id\":\"$company\",\"custom_fields\":{}}" "$APHEL_URL/api/v1/contacts" | jq -r .data.id)
curl -s -X POST -H "Authorization: Bearer $APHEL_TOKEN" -H "Content-Type: application/json" \
  -d "{\"title\":\"Acme onboarding\",\"amount\":\"15000.00\",\"currency\":\"BRL\",\"company_id\":\"$company\",\"contact_id\":\"$contact\",\"custom_fields\":{}}" \
  "$APHEL_URL/api/v1/deals"
```

Compatibility: contract-breaking changes only ship in `/api/v2` (policy in the repository, `docs/api/versioning.md`).
