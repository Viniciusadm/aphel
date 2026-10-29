# API style guide

Rules that every `/api/v1` resource follows. They apply to whoever writes routes in the backend and are the promise made to whoever integrates with Aphel. The OpenAPI document at `/api/openapi.json` is the source of truth for the contract; this guide explains the conventions it embodies.

## Basics

- `/api/v1` prefix, JSON in UTF-8, property names in `snake_case`.
- Resources in the **plural** and paths in `kebab-case` (`/contacts`, `/pipeline-stages`, `/lost-reasons`). Non-CRUD actions become sub-resources with `POST` (`POST /deals/{id}/won`, `POST /contacts/{id}/archive`).
- IDs are UUID v7 as text. Enumerations are strings in `snake_case`.
- A `POST` that creates responds **201**; `DELETE` responds **204** with no body; other successes respond **200**.

## Authentication

- **Browser:** `aphel_session` session cookie. Non-`GET` requests require `X-Requested-With: aphel`, except on public routes (`/api/v1/public/…` and `/api/v1/hooks/…`).
- **Integrations:** `Authorization: Bearer aphel_<prefix>_<secret>`, created in **Settings → API tokens**. With Bearer the cookie is ignored and the CSRF header is not required.
- Scopes `<resource>:read` / `<resource>:write`, derived from permissions (`contacts.read` → `contacts:read`; `contacts.create|update|delete|archive|export` → `contacts:write`; `settings.manage` → `settings:write`). A token never has more power than the user who created it **at the time of use**: the effective permission is the intersection of the scopes with the creator's current permissions. Routes that require no permission (e.g. listing tags, pipelines) accept any valid token.
- Session and account routes (`/auth/*`, `/api-tokens`) do **not** accept tokens (403 `api_token_not_allowed`).
- Invalid, revoked or expired token, or one whose creator is deactivated → 401 `invalid_api_token`. Insufficient scope → 403 `insufficient_scope`.
- Per-token limit in a sliding window (default 600 requests/60 s, adjustable in Settings → API tokens → Limits). Every Bearer response carries `X-RateLimit-Limit`, `X-RateLimit-Remaining` and `X-RateLimit-Reset` (seconds until a slot frees up); when exceeded, 429 `rate_limited` with `Retry-After`.

## Responses

- **Item:** `{ "data": { ... } }` on `GET /resource/{id}`, and on `POST`, `PATCH` and `PUT` that return the resource (type `Data<T>` in `kernel/api/dto/common.rs`).
- **Paginated list:** `{ "data": [ ... ], "meta": { "total": 120, "page": 1, "per_page": 25 } }` (type `Page<T>`).
- **Cursor list** (timeline, notifications): `{ "data": [ ... ], "meta": { "next_cursor": "..." | null } }`.
- Small, fixed collections without pagination (catalogs, ordering) also use `{ "data": [ ... ] }`.

## Listings

- `page` (≥ 1, default 1) and `per_page` (1–100, default 25); outside that, 422 `validation_error`.
- `filter`: the AST as URL-encoded JSON (`filter={"op":"and","conditions":[...]}`, see [`custom-fields.md`](../architecture/custom-fields.md)) **or**, for simple cases, `filter[field][operator]=value` (`filter[status][eq]=open`, `filter[amount][gte]=1000`, `filter[stage_id][in]=a,b`, `filter[email][is_empty]=`, `filter[company_id][in_group]=<id>` for records of the company and all its subsidiaries); `filter[field]=value` means `eq`. Multiple conditions go into an `and` group; both forms together are also combined with `and`. Invalid field or operator → 422 with `details.fields`.
- `sort=field,-other` (up to 3; `-` = descending). Without `sort`, the resource's documented default order.
- `search=` text search when the resource supports it.
- `include=rel1,rel2` loads the relationships allowed by the resource (documented on the parameter); the relationship appears embedded in the item under its own name (`"company": { ... }`). Unknown relationship → 422 on the `include` field.

## Writing

- `PATCH` is **partial**: only the fields sent change; `null` clears an optional field. `PUT` only to replace entire collections (ordering, layout, preferences).
- **Concurrency:** every editable resource exposes `updated_at`. `PATCH` accepts `If-Match: <updated_at>` (with or without quotes, ISO-8601); if the record has changed since then, 409 `version_conflict` (reread the resource and reapply the change). Without the header, the write is unconditional.
- Custom fields live in `custom_fields: { "<key>": value }`, on read and on write.
- `Idempotency-Key: <up to 255 characters>` is accepted on every `POST`: the same key with the same body, within 24 h, returns the stored response (same status and body, header `Idempotent-Replayed: true`); the same key with a different body → 422 `idempotency_key_reused`; while the first request is still in progress → 409 `idempotency_key_in_progress`. The key is scoped per credential (session or token).

## Formats

- Dates and times in ISO-8601 **UTC** with `Z` (`2026-10-20T13:45:00Z`); dates without time as `YYYY-MM-DD`.
- Money as a **decimal string** with two decimal places + ISO 4217 currency: `"amount": "15000.00", "currency": "BRL"`. Writes accept a string or a number.
- Phone numbers in E.164; emails in lowercase.

## Errors

Single envelope: `{ "error": { "code", "message", "details" } }`. `code` is stable and documented in [`errors.md`](errors.md); `message` is meant for people and may change.

| Status | When | Typical codes |
| --- | --- | --- |
| 401 | No credential or invalid credential | `unauthorized`, `invalid_api_token` |
| 403 | No permission, scope, CSRF | `permission_denied`, `insufficient_scope`, `csrf_header_missing`, `api_token_not_allowed` |
| 404 | Resource does not exist or is not visible | `<resource>_not_found` |
| 409 | State or version conflict | `version_conflict`, `idempotency_key_in_progress`, resource codes |
| 413 | Body above the route's limit | `payload_too_large`, resource codes |
| 422 | Validation, including malformed JSON body | `validation_error` (with `details.fields[]` when it points to fields), resource codes |
| 429 | Rate limit | `rate_limited` |

## Documentation in the code

Every route has `#[utoipa::path]` with `tag`, `operation_id`, a description of what it does and the permission required, parameters (`params(...)`), body and **all** responses, including errors (`body = ErrorEnvelope`) with the possible codes in the description.

No 4xx/5xx response goes out outside the envelope: axum rejections (malformed body, body too large) are converted by the `JsonBody` extractor and the `payload_limit::envelope` layer. In the documentation:

- Routes without login (`/auth/*`, `/installer`, public links) also describe 403 `csrf_header_missing` when they accept `POST` from the browser, 429 `rate_limited` when they have a limit (with the limit and the window) and 503 `configuration_error` when they depend on the database.
- Session routes document 401 (missing session) and 403 `api_token_not_allowed`.
- Responses that do not reveal whether an email exists (password recovery, login lockout) say so in the description, so that nobody “fixes” the behavior. DTOs have `#[schema(example = ...)]` on non-obvious fields. The route goes into `/api/openapi.json` and `back/openapi.json` is regenerated (CI fails if it diverges from the generated one).

## Per-resource checklist

- [ ] Plural path, actions as sub-resources
- [ ] Item in `{ data }`, list in `{ data, meta }`
- [ ] `page`/`per_page` (max. 100), `filter` (JSON and `filter[field][op]`), `sort`, `include` when there are relationships
- [ ] Partial `PATCH` with `If-Match` → 409 `version_conflict`
- [ ] `custom_fields` on records with custom fields
- [ ] UTC ISO-8601 dates, money as decimal string + currency
- [ ] Errors with stable codes listed in `errors.md`
- [ ] `utoipa` with description, examples and error responses


Ready-made clients: [TypeScript SDK and cURL and PHP examples](sdk.md).
