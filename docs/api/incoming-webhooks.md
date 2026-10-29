# Incoming webhooks

An incoming webhook is a **secret URL** that receives `POST` requests from external systems (website form, landing page, automation tool, another CRM) and runs, in order, actions configured without code: create or update a contact, create a deal and log an activity. Every request received is kept in the log with the result of each action.

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

`actions` is a list (up to 10) run in order, **as the user who created the webhook**: their permissions are reloaded on every request. If that user has been deactivated, the request fails with 403 `incoming_webhook_owner_unavailable`; if they lost the permission for an action, 403 `incoming_webhook_permission_denied`. The actions use the same use cases as the API, so they generate events (`contact.created`, `deal.created`, ...), audit entries and trigger outgoing webhooks normally.

| `type` | Configuration | Behavior |
| --- | --- | --- |
| `upsert_contact` | `match_by`: `"email"`, `"phone"`, `"field"` or `null`; `match_field` (only with `"field"`); `fields` | Looks for an existing contact and, if found, updates only the fields that came in; otherwise creates one. `"email"`: same mapped email. `"phone"`: the mapped `phone` or `mobile` matches the contact's phone or mobile (normalized to E.164). `"field"`: same value in the contact's unique custom field `match_field`, which must be mapped in `fields`. `first_name` is required in the mapping |
| `create_deal` | `pipeline_id`, `stage_id`, `fields` | Creates the deal in the stage; `title` is required. Links it to the contact from the previous action if `contact_id` is not mapped |
| `create_activity` | `activity_type_id`, `fields` | Creates the activity; `title` is required. Links it to the deal created earlier (and to the contact) or only to the contact |

`GET /api/v1/incoming-webhooks/targets` lists, per action type, the mappable fields: the system ones and the record type's **active custom fields** (by key). The configuration is validated on save: an unknown field, malformed path, missing required field, stage outside the pipeline or inactive activity type respond 422 pointing to `actions.<i>...`.

Mappable system fields:

- **Contact:** `first_name`, `last_name`, `email`, `phone`, `mobile`, `job_title`, `company_id`, `owner_id`, `source`, `notes`, `address_street`, `address_number`, `address_complement`, `address_district`, `address_city`, `address_state`, `address_postal_code`, `address_country`.
- **Deal:** `title`, `amount`, `currency`, `probability`, `expected_close_date`, `company_id`, `contact_id`, `owner_id`, `source`.
- **Activity:** `title`, `description`, `due_at`, `priority`, `duration_minutes`, `owner_id`.

### Mapping

Each entry in `fields` is `"<CRM field key>": <expression>`:

| Expression | Result |
| --- | --- |
| `"$.email"`, `"$.lead.contato.email"` | Value at the path in the received body (`$` is the root) |
| `"$.itens[0].nome"`, `"$['key with dot.x']"` | Array index and bracketed key |
| `"Lead do site: {{ $.name }}"` | Text with `{{ $.path }}` snippets substituted (empty if missing) |
| `"Site"`, `10`, `true` | Constant |

- Nonexistent path or `null` → the field is ignored (it never erases an existing value).
- System fields receive text; numbers and booleans become text, arrays become comma-separated items.
- Custom fields receive the JSON value as it came and go through the type's normal validation (select option, number, date...).
- A value rejected by the use case → 422 `webhook_mapping_failed` with `details.fields[].field` in the format `actions.<i>.<field>`. Previous actions are **not** undone (each one has its own transaction) and the log shows the partial result.

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
    "message": "action 1 (create_deal) failed: invalid deal",
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

## Example: website form → contact + deal

Configuration (`POST /api/v1/incoming-webhooks`):

```json
{
  "name": "Formulário do site",
  "actions": [
    {
      "type": "upsert_contact",
      "match_by": "email",
      "fields": {
        "first_name": "$.nome",
        "email": "$.email",
        "mobile": "$.telefone",
        "source": "Site"
      }
    },
    {
      "type": "create_deal",
      "pipeline_id": "<pipeline id>",
      "stage_id": "<initial stage id>",
      "fields": {
        "title": "Site: {{ $.nome }}",
        "source": "Site"
      }
    }
  ]
}
```

Submitting a plain HTML form:

```html
<form method="post" action="https://crm.exemplo.com.br/api/v1/hooks/whk_...">
  <input name="nome" required />
  <input name="email" type="email" required />
  <input name="telefone" />
  <button>Enviar</button>
</form>
```

Or with cURL:

```bash
curl -X POST "https://crm.exemplo.com.br/api/v1/hooks/$TOKEN" \
  -H 'Content-Type: application/x-www-form-urlencoded' \
  -H "X-Aphel-Delivery: form-$(date +%s%N)" \
  --data-urlencode 'nome=Ana Souza' \
  --data-urlencode 'email=ana@acme.com.br' \
  --data-urlencode 'telefone=(11) 98888-7777'
```

The contact is created (or updated, if the email already exists), the deal is created linked to it and the `deal.created` event goes out through the signed outgoing webhooks like any other creation.
