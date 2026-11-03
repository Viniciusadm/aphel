# Incoming webhooks

An incoming webhook is a **secret URL** that receives `POST` requests from external systems (website form, landing page, automation tool, another CRM) and runs, in order, actions configured without code: create or update records in enabled entities. Every request received is kept in the log with the result of each action.

Managed in **Settings → Incoming webhooks** or through the API at `/api/v1/incoming-webhooks` (permission `settings.manage`, session or API token with `settings:write`).

## URL

```
POST https://<your-aphel>/api/v1/hooks/<token>
```

- The token (`whk_` + 64 hexadecimal characters) is shown **only once**: on creation and when regenerating it (`POST /api/v1/incoming-webhooks/{id}/regenerate-token`). Aphel stores only the hash; the listing shows only the prefix (`token_prefix`). Regenerating invalidates the previous URL immediately.
- The route uses neither a session, an API token nor the `X-Requested-With` header: whoever knows the URL can send requests. Treat it like a password and, if possible, enable signing.
- A deactivated webhook (`is_active = false`) responds 403 `incoming_webhook_disabled` (the request is logged). An unknown token responds 404 `incoming_webhook_not_found` (not logged).
- Limit of 120 requests per minute per URL (sliding window, per instance). Above that it responds 429 `rate_limited` with `Retry-After` (the request is not logged).

## Accepted body

| `Content-Type` | Conversion |
| --- | --- |
| `application/json` (or `*+json`, `text/plain`, missing) | JSON object or array; any other value → 422 `invalid_webhook_payload` |
| `application/x-www-form-urlencoded` | Becomes an object `{ "field": "value" }`; repeated keys (or `field[]`) become an array |
| Others | 422 `invalid_webhook_payload` |

Limit of **256 KB** per body; above that, 413 `webhook_payload_too_large`.

## Signature (optional)

When creating with `generate_signing_secret: true` or via `POST /api/v1/incoming-webhooks/{id}/signing-secret`, Aphel generates a `whsec_...` secret shown **only once** (it is stored encrypted in the vault; the API only reports `secret_configured`). From then on every request needs:

```
X-Aphel-Signature: t=<unix_timestamp>,v1=<hex(hmac_sha256(secret, "<t>.<raw body>"))>
```

- Same format as Aphel's outgoing webhooks. The HMAC key is the secret exactly as displayed (including `whsec_`).
- The HMAC is computed over the **exact bytes** of the body sent.
- `t` must be at most `signature_tolerance_seconds` (default 300 s, between 30 and 86400) away from the server clock.
- Multiple `v1=` values are accepted (useful during secret rotation); one match is enough. The comparison is constant-time.
- Missing, malformed, wrong or out-of-window signature → 401 `invalid_webhook_signature`.
- `DELETE /api/v1/incoming-webhooks/{id}/signing-secret` removes the secret and goes back to accepting unsigned requests.

Shell example:

```bash
BODY='{"name":"Ana Souza","email":"ana@acme.com.br"}'
TS=$(date +%s)
SIG=$(printf '%s.%s' "$TS" "$BODY" | openssl dgst -sha256 -hmac "$APHEL_WEBHOOK_SECRET" -hex | sed 's/^.* //')
curl -X POST "https://crm.exemplo.com.br/api/v1/hooks/$TOKEN" \
  -H 'Content-Type: application/json' \
  -H "X-Aphel-Signature: t=$TS,v1=$SIG" \
  -d "$BODY"
```

## Replay protection

- If the request carries `X-Aphel-Delivery: <unique id>` (up to 200 characters), the (webhook, id) pair is kept for **24 h**; another request with the same id responds 409 `webhook_replayed`.
- On signed webhooks without `X-Aphel-Delivery`, the signature itself (`t` + `v1`) plays that role.
- Without a signature and without `X-Aphel-Delivery`, there is no replay protection: deduplication is up to the action (e.g. contact matched by email).
- When no action could run (status `failed`), the id is released so the sender can try again. On full or partial success it stays marked.
- The `incoming_webhooks.purge` job (hourly) deletes marks older than 24 h and log entries older than 30 days.

## Actions

Actions run in order as the webhook creator, with current permissions reloaded for each request. The target must have an enabled owner and a registered writable handler. `GET /api/v1/incoming-webhooks/targets` returns available action/entity pairs and editable active fields.

- `create_record`: `record_type`, `fields`, optional `assignment_rule_id`.
- `upsert_record`: the same fields plus `match_field`, which must identify a mapped editable, filterable, unique field. The creator needs read, create and update permissions. A scoped query finds the existing record; multiple visible matches fail instead of choosing an arbitrary record.

Writes use registered handlers, including owner validation, record scopes, custom fields, audit and events. Owner-specific required inputs, workflow stages and other references are ordinary mapped fields validated on execution. Assignment is optional and requires an available selector for the target entity and an editable owner field. There are no implicit links between business types or legacy action aliases.

### Mapping

`fields` maps logical entity field keys to expressions. System and custom fields preserve their JSON value types. The mapping root contains `payload` (the received body) and `actions` (successful earlier action identities).

- `$.payload.email`: incoming value.
- `$.payload.items[0].name`: incoming array value.
- `$.actions[0].id`: ID produced by the first action.
- `Received: {{ $.payload.name }}`: text template.
- Scalar strings, numbers and booleans: constants.

Missing paths are ignored. Mapped values pass through the target's normal validation. Actions commit separately: earlier successful actions remain committed when a later one fails, and subsequent actions are skipped. Replay protection does not establish exactly-once writes across crashes or unrelated deliveries. Unique upserts remain scoped and use the owner's normal uniqueness constraints.

## Responses

Success:

```json
{ "data": { "request_id": "0192a1b2-...", "status": "succeeded" } }
```

Failure (the `request_id` also comes in `details` to locate it in the log):

```json
{
  "error": {
    "code": "webhook_mapping_failed",
    "message": "action 1 (create_record) failed: invalid field values",
    "details": {
      "request_id": "0192a1b2-...",
      "fields": [{ "field": "actions.1.amount", "message": "must be a decimal number" }]
    }
  }
}
```

| HTTP | Code | When |
| --- | --- | --- |
| 200 | — | All actions ran |
| 401 | `invalid_webhook_signature` | Signature missing, wrong or outside the window |
| 403 | `incoming_webhook_disabled` | Webhook deactivated |
| 403 | `incoming_webhook_owner_unavailable` | Creator deactivated |
| 403 | `incoming_webhook_permission_denied` | Creator lacks the permission for an action |
| 404 | `incoming_webhook_not_found` | Unknown token |
| 409 | `webhook_replayed` | Delivery repeated within 24 h |
| 413 | `webhook_payload_too_large` | Body above 256 KB |
| 422 | `invalid_webhook_payload` | Unsupported `Content-Type` or malformed body |
| 422 | `webhook_mapping_failed` | An action rejected the mapped values |
| 500 | `internal_error` | Internal error (details only in the server log) |

## Log

`GET /api/v1/incoming-webhooks/{id}/requests?status=succeeded|partial|failed|rejected` lists the requests from the last 30 days; `GET .../requests/{request_id}` returns the detail. Each entry stores: date, IP, `Content-Type`, allowlisted headers (`content-type`, `content-length`, `user-agent`, `origin`, `referer`, `x-forwarded-for`, `x-real-ip`, `x-request-id`, `x-aphel-delivery`, `x-aphel-event`, `x-github-event`, `x-github-delivery`; never `Authorization`, `Cookie` or `X-Aphel-Signature`), body truncated at 8 KB, status, returned HTTP status, error code and, per action, `succeeded`/`failed`/`skipped` with the created or updated record.

| Status | Meaning |
| --- | --- |
| `succeeded` | All actions ran |
| `partial` | Some action ran and one failed (the following ones are `skipped`) |
| `failed` | No action ran (creator unavailable, first action failed, internal error) |
| `rejected` | Refused before the actions (deactivated, signature, replay, invalid or oversized body) |

## Example: create a record from a received payload

```json
{
  "name": "External intake",
  "actions": [
    {
      "type": "create_record",
      "record_type": "custom.request",
      "fields": { "title": "$.payload.subject" }
    }
  ]
}
```

The entity must already exist and allow the creator to create records. Use the targets catalog to choose its editable fields. A following action can map a reference field to `$.actions[0].id`.
