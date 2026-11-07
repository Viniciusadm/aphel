---
title: Settings and secrets
description: Let administrators configure a plugin and store API keys safely, with setting types, validation, permissions and the secrets vault.
sidebar:
  order: 10
---

Settings let an administrator configure a plugin after it is installed, without changing its package. They are declared in the manifest, edited in the interface or through the API, and read by the backend.

```toml
settings_permission = "library.configure"

[[permissions]]
key = "library.configure"
label = "Configure the library plugin"

[[settings]]
key = "loan_days"
label = "Loan period (days)"
type = "number"
default = 14
min = 1
max = 90
group = "Loans"
help = "How long a book can be borrowed."

[[settings]]
key = "notify_overdue"
label = "Notify when overdue"
type = "boolean"
default = true
group = "Loans"

[[settings]]
key = "currency_display"
label = "Price display"
type = "select"
default = "symbol"
options = [
  { value = "symbol", label = "Symbol" },
  { value = "code", label = "ISO code" },
]
group = "Display"

[[settings]]
key = "catalog_url"
label = "Catalog URL"
type = "text"
group = "Catalog"

[[settings]]
key = "catalog_token"
label = "Catalog API token"
type = "secret"
group = "Catalog"
```

## Setting types

| `type` | Value | Validation when saved |
| --- | --- | --- |
| `number` | Integer | Must be an integer inside `min` and `max` when set. Decimals are not accepted. |
| `boolean` | `true` or `false` | Must be a boolean. |
| `text` | String | At most 1000 characters. |
| `select` | String | Must be the `value` of one of `options`. `options` is required for `select` and not allowed for other types. A `default` must be one of the options. |
| `secret` | String, write-only | Non-empty text of at most 8 KiB, or `null` to remove. Cannot have a `default`. |

Other keys: `label` (required), `help` (text under the input), `default` (not for secrets) and `group`. There is no `group` setting type: `group` is a text label. In the interface, a heading with that text appears before the first setting of each run of settings that share the same group, so keep settings of the same group next to each other.

Keys are lowercase, start with a letter and have at most 64 characters. A setting has no `required` flag; if your plugin needs a value, check for it and fail with a clear plugin error.

## Who can change settings

An administrator with `system.manage` can always read and change settings. Set `settings_permission` to a permission declared in your own `[[permissions]]` to let other users do it as well. Anything else answers `403 permission_denied`.

## Where they are edited

In **Settings**, **Integrations**, **Plugins**, each installed plugin with settings has a settings dialog. Numbers use a number input with the `min` and `max`, booleans a checkbox, selects a dropdown, text a text input and secrets a password input that shows whether a value is configured and offers to clear it. Leaving a non-secret input empty restores the default.

## API

| Endpoint | Description |
| --- | --- |
| `GET /api/v1/wasm-runtime/{key}/settings` | The settings with their current values. |
| `PUT /api/v1/wasm-runtime/{key}/settings` | Saves settings. Body: a JSON object of setting key to value. |

A GET answers `{ "data": [...] }`. Each item has this shape.

```json
{
  "key": "loan_days",
  "label": "Loan period (days)",
  "type": "number",
  "help": "How long a book can be borrowed.",
  "min": 1,
  "max": 90,
  "group": "Loans",
  "options": [],
  "value": 14,
  "configured": null
}
```

`value` is the stored value, or the default when nothing is stored, or `null`. For a secret, `value` is always `null` and `configured` is `true` or `false`. For other types `configured` is `null`.

A PUT validates every key you send against the declarations:

- A key that is not a setting of the plugin is refused: `is not a setting of this plugin`.
- A value of the wrong type or outside the limits: `must be a valid <type>`.
- A secret value that is not a non-empty text of at most 8 KiB (or `null`): `must be a non-empty text or null`.
- All problems are returned together as `422 plugin_settings_invalid` with a field entry per key.

Semantics: the non-secret settings you send **replace** the stored ones, so a non-secret key you leave out falls back to its default. `null` also restores the default. For secrets, a string stores a new secret, `null` removes it and leaving the key out keeps the current secret. The answer is the updated plugin. A successful PUT publishes the event `plugin.settings_updated` and writes an audit entry; secret values are recorded as `[redacted]`.

## Reading settings in the backend

The host function `settings-json` returns the effective settings as one JSON object: the defaults declared in the manifest overlaid with the stored values. Secrets are never included.

```js
const days = ctx.setting('loan_days') ?? 14
const all = ctx.settings()
```

In Rust, `ctx.setting::<i64>("loan_days")?` and `ctx.settings()?`.

The values the backend sees are those of the plugin catalog snapshot. A change made through the API reaches a running process after the catalog refresh, which is checked every few seconds (at most every two seconds per process).

## Secrets

Use a `secret` setting for anything you must not expose: API keys, tokens, signing secrets.

- Secrets are stored in the organization's secrets vault, encrypted with `APP_ENCRYPTION_KEY`, under the owner `plugin` / plugin key and the setting key as the name.
- They are never returned by the API: the settings endpoints only say whether a secret is configured. They are not written to logs, and audit entries record `[redacted]`.
- The backend reads one with `secret(key)` (JavaScript `ctx.secret('catalog_token')`, Rust `ctx.secret("catalog_token")?`). The result is the string, or `null` (`None` in Rust) when nothing is configured. Asking for a key that is not a declared `secret` setting fails with `404 plugin_secret_not_declared`.
- `secret` is available in every entry point, including hooks.
- Uninstalling the plugin deletes its secrets.

Handle secrets in plugin code with care. The plugin is trusted with its own secrets: it can send them to any host in `[http] allowed_hosts` or return them in a route response. A deferred effect stores its command as JSON in the database until it completes, so a secret that you put in the header of a deferred `http` effect is stored there in clear. Prefer calling the external service from a route (where `http-fetch` is allowed) when a request needs a secret and its result can wait.

The vault is covered in [Security model](/docs/plugins/security-model/) and for operators in [Security](/docs/admin/security/).


## Client certificates (mTLS)

Use a `tls_identity` setting when an HTTPS endpoint requires a client certificate. The backend uses the identity on behalf of the plugin; `ctx.secret` cannot read it.

```toml
[http]
allowed_hosts = ["api.example.com"]

[[settings]]
key = "partner_certificate"
label = "Partner client certificate (PEM)"
type = "tls_identity"
tls_origins = ["https://api.example.com"]
```

`tls_origins` must contain at least one exact HTTPS origin, including a port if it differs from 443. Wildcards, credentials, paths other than `/`, queries and fragments are not accepted. The request must also pass `http.allowed_hosts` and the installation's outbound network policy. Redirects remain disabled.

An operator with access to the plugin settings uploads a `.pem` file in the settings dialog, or sends its contents as the setting value to `PUT /wasm-runtime/{key}/settings`. The PEM must contain the client certificate chain (leaf first) and its matching unencrypted private key, up to 8 KiB total. The backend validates that it can configure a TLS client before saving. No default is allowed. As with secrets, omitting the key preserves it, a string replaces it, and `null` revokes it. API responses only return `configured`; the material is encrypted at rest, with audit values redacted.

The credential uses a separate vault owner, `plugin_tls` / plugin key, so changing a setting to `secret` cannot expose a previously stored private key. It is never included in settings or job payloads and is removed on uninstall. Each request resolves the credential again and constructs its own TLS client: replacement or removal affects subsequent requests and job attempts without restarting. An already established connection is not revoked retroactively. Certificate expiry and acceptance are checked during TLS negotiation with the remote server; there is no expiry-alert UI in this implementation.

PFX/PKCS#12 files are not accepted directly by the current rustls-based client. Convert an A1 to PEM before upload; OpenSSL prompts for the PFX password:

```sh
openssl pkcs12 -in certificado.pfx -out identidade.pem -nodes
```

Keep the resulting private-key file restricted to the operator and remove it when no longer needed. This configures connection authentication, not XML/document signing.

JavaScript:

```js
const response = ctx.http.send({
  method: 'POST',
  url: 'https://api.example.com/documents',
  clientIdentity: 'partner_certificate',
  body: xml,
})
```

Rust:

```rust
let response = ctx.http().send(
    HttpRequest::new("POST", "https://api.example.com/documents")
        .client_identity("partner_certificate")
        .body(xml),
)?;
```

For durable HTTP effects, use the snake-case JSON field `client_identity`:

```js
ctx.deferEffect('send-document', {
  kind: 'http',
  method: 'POST',
  url: 'https://api.example.com/documents',
  client_identity: 'partner_certificate',
  body: xml,
})
```

The job stores only the reference and checks the current declaration, destination and credential on every attempt. HTTP effects have a 30-second attempt timeout and persist the result. Non-2xx responses and unknown outcomes must be handled explicitly by polling or a continuation. A timeout can leave the remote outcome unknown: implement reconciliation and use idempotency supported by the destination. The HTTP effect adds its existing stable `Idempotency-Key`; a remote endpoint must honor it for deduplication.
