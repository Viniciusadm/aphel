# Outgoing webhooks

An outgoing webhook sends Aphel's [public events](events.md) to a URL of yours, with a signed JSON `POST`. Use it to keep another system (ERP, BI, automations) up to date without polling the API all the time.

Managed in **Settings → Outgoing webhooks** or through the API at `/api/v1/webhook-endpoints` (permission `settings.manage`; session or API token with `settings:write`). The installation can have up to 20 endpoints. Deliveries are made by the worker (`aphel worker` or `aphel serve --with-worker`).

## Endpoints

| Method and path | What it does |
| --- | --- |
| `GET /webhook-endpoints` | Lists the endpoints |
| `POST /webhook-endpoints` | Creates (`url`, `description`, `events`, `is_active`) and returns the signing secret only once |
| `GET /webhook-endpoints/{id}` | One endpoint |
| `PATCH /webhook-endpoints/{id}` | Updates (partial, accepts `If-Match`); `is_active: true` reactivates and resets the failure counter |
| `DELETE /webhook-endpoints/{id}` | Deletes the endpoint, the secret and the delivery history |
| `POST /webhook-endpoints/{id}/rotate-secret` | Generates a new secret (shown only once); the previous one stops working immediately |
| `POST /webhook-endpoints/{id}/test` | Immediately sends a synthetic `webhook.test` event (10 s timeout) and responds with the recorded delivery and its result (`status`, `response_status`, `error`) |
| `GET /webhook-endpoints/{id}/deliveries?status=&page=&per_page=` | Delivery history, most recent first |
| `GET /webhook-endpoints/{id}/deliveries/{delivery_id}` | One delivery, with the exact body sent (`request_body`) and the beginning of the response |
| `POST /webhook-endpoints/{id}/deliveries/{delivery_id}/redeliver` | Resends the same body (same event `id`) in a new delivery |

`events` is the list of types from `GET /api/v1/events/catalog` (e.g. `["deal.won", "contact.created"]`) or `["*"]` for all, including those added to the catalog later. Unknown types → 422 `validation_error` on the `events` field.

## Signing secret

Aphel generates the secret (`whsec_` + 64 hexadecimal characters) on creation and on `rotate-secret`, and shows it **only in those responses** (`signing_secret`). It is stored encrypted in the secret vault (requires `APP_ENCRYPTION_KEY`; without the key, creation responds 503) and never comes back through the API, logs or audit: responses only include `secret_configured`. Lost the secret? Generate another one.

## Request sent

```
POST <endpoint url>
Content-Type: application/json
User-Agent: Aphel-Webhooks/1.0
X-Aphel-Event: deal.won
X-Aphel-Delivery: 0192f0c5-1a2b-7c3d-8e4f-5a6b7c8d9e0f
X-Aphel-Signature: t=1761055392,v1=5f2b6c...e91a
```

The body is the envelope described in [Event catalog](events.md#delivered-body) (`id`, `type`, `actor`, `occurred_at`, `data`, `previous`). It is built **once**, when the event is processed, and stored with the delivery: retries and redeliveries send exactly the same bytes.

| Header | Content |
| --- | --- |
| `X-Aphel-Event` | Event type (`webhook.test` for the test event) |
| `X-Aphel-Delivery` | Delivery id. It changes on each manual redelivery; automatic retries of the same delivery keep the id |
| `X-Aphel-Signature` | `t=<unix timestamp>,v1=<hex>` where `v1` = HMAC-SHA256 of the text `<t>.<body>` with the endpoint's secret. `t` is the time of the attempt |

Delivery **is at least once**: discard repetitions by the body's `id` (the event id), not by `X-Aphel-Delivery`.

### Verifying the signature

1. Read the **raw** body (bytes exactly as they arrived, before any JSON parsing).
2. Split `t` and `v1` from the `X-Aphel-Signature` header.
3. Reject if `|now - t|` exceeds your tolerance (suggestion: 5 minutes), to prevent replay.
4. Compute `HMAC-SHA256(secret, t + "." + body)` in lowercase hexadecimal and compare it with `v1` in constant time.

Node.js (Express):

```js
import crypto from 'node:crypto'
import express from 'express'

const app = express()
const secret = process.env.APHEL_WEBHOOK_SECRET

app.post('/webhooks/aphel', express.raw({ type: 'application/json' }), (req, res) => {
  const header = req.get('X-Aphel-Signature') ?? ''
  const parts = Object.fromEntries(header.split(',').map((part) => part.split('=')))
  const timestamp = Number(parts.t)
  if (!timestamp || Math.abs(Date.now() / 1000 - timestamp) > 300) return res.sendStatus(400)

  const expected = crypto.createHmac('sha256', secret).update(`${parts.t}.${req.body}`).digest('hex')
  const received = Buffer.from(parts.v1 ?? '', 'hex')
  if (received.length !== 32 || !crypto.timingSafeEqual(received, Buffer.from(expected, 'hex'))) {
    return res.sendStatus(401)
  }

  const event = JSON.parse(req.body.toString('utf8'))
  // process event.type / event.data and discard repeated event.id
  res.sendStatus(204)
})
```

Python (Flask):

```python
import hashlib, hmac, os, time
from flask import Flask, request, abort

app = Flask(__name__)
SECRET = os.environ["APHEL_WEBHOOK_SECRET"].encode()

@app.post("/webhooks/aphel")
def aphel_webhook():
    body = request.get_data()
    parts = dict(item.split("=", 1) for item in request.headers.get("X-Aphel-Signature", "").split(",") if "=" in item)
    timestamp = parts.get("t", "")
    if not timestamp.isdigit() or abs(time.time() - int(timestamp)) > 300:
        abort(400)
    expected = hmac.new(SECRET, timestamp.encode() + b"." + body, hashlib.sha256).hexdigest()
    if not hmac.compare_digest(expected, parts.get("v1", "")):
        abort(401)
    event = request.get_json()
    return "", 204
```

## Response, retries and deactivation

- Any **2xx** response is a success. Any other code, network error, timeout (default 10 s, adjustable in Settings → Webhooks → Delivery) or redirect (not followed) is a failure. Respond quickly and process asynchronously on your side.
- Aphel stores the HTTP code, the duration and the first **2 KB** of the response.
- Failures are retried up to **7 attempts** in total, waiting 1 min, 5 min, 30 min, 2 h, 6 h and 12 h between them (about 21 h overall). After that the delivery becomes `failed`.
- Each delivery that fails for good adds 1 to the endpoint's `consecutive_failures` counter; any success resets it. When the consecutive failure limit is reached (default 15, in Settings → Webhooks → Delivery) the endpoint is **automatically deactivated** (`disabled_reason: "failures"`, internal event `webhook_endpoint.disabled`) and whoever has `settings.manage` receives a notification. Reactivate it with `PATCH { "is_active": true }` or through the screen, after fixing the destination.
- Deliveries of a deactivated endpoint that were still pending are closed as `failed` without a new send. Use **Redeliver** after reactivating.
- The test event and manual redeliveries are made in a single attempt and do not count toward automatic deactivation.
- The history is kept for the days configured in Settings → Webhooks → Delivery (default 30). A maintenance job every 10 minutes also re-enqueues pending deliveries whose job was lost.

## SSRF protection

- The URL must be an absolute `http` or `https` URL, with no user/password or fragment. With the https requirement turned on in Settings → Webhooks → Delivery (the installer turns it on when the public address is https) only `https` is accepted.
- The host is resolved on registration and **again on each send**. Rejected: loopback, private networks (10/8, 172.16/12, 192.168/16, `fc00::/7`), link-local (169.254/16, `fe80::/10`), CGNAT (100.64/10), unspecified addresses, multicast, broadcast, documentation, reserved ranges, IPv4-mapped IPv6 that falls into those ranges, and the names `localhost`, `*.local` and `*.internal`. If any resolved address is blocked, the URL is rejected (422 `webhook_url_not_allowed` on registration; attempt failure on send).
- The connection uses the result of that same filtered resolution (the HTTP client's own resolver), with no proxy and without following redirects, which prevents DNS rebinding and redirection to the internal network.
- Installations that need to call services on their own network turn on `WEBHOOKS_ALLOW_PRIVATE_NETWORKS=true` (see [Configuration](../configuration.md#outgoing-webhooks)).

## Audit and internal events

Creating, updating, deleting, generating a secret, testing and redelivering write audit entries and internal events (`webhook_endpoint.created|updated|deleted|secret_rotated|tested|disabled`, `webhook_delivery.redelivered`). These events are not part of the public catalog and are not delivered by webhook. The secret never appears in the audit (`signing_secret` is recorded as `"[redacted]"`).

## In the code

Use cases (`service.rs`, functions over `Ctx`), delivery/maintenance (`delivery.rs`), the outbox consumer (`webhooks.outgoing`) and the jobs (`webhook.deliver`, `webhooks.maintenance`) in `runtime.rs` and SQL in `store/` in the `webhooks` module (`back/modules/webhooks/src/outgoing`); signing, transport (`WebhookSender`, `WebhookUrlGuard`) and network protection live in the kernel (`back/kernel/src/application/webhooks/outgoing`, `back/kernel/src/infrastructure/webhooks/outgoing`), because automations and IMAP also use them; routes in `back/modules/webhooks/src/outgoing/http.rs`; screen in `web/src/modules/webhooks/outgoing`.
