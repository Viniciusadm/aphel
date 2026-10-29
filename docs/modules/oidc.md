# "Single sign-on (OIDC)" module (`oidc`)

Single sign-on (SSO) with any corporate **OpenID Connect** provider (Okta, Keycloak, Auth0, Microsoft Entra ID, Google Workspace via generic OIDC…). The account must **already exist** in Aphel: single sign-on only links the email verified by the provider to an active user of the installation; nothing is created automatically.

The `aphel-mod-oidc` crate (`back/modules/oidc`) is also the **shared library** of the [`google`](google.md) and [`microsoft`](microsoft.md) modules, which depend on it as a crate (alias `oidc`) and in the manifest (`depends_on("oidc")`):

| Module | What it contains |
| --- | --- |
| `oidc::sso::oauth` | OAuth2 client (authorization code + PKCE S256), code exchange and refresh at the token endpoint, reading and validating the id_token claims, OIDC discovery (`.well-known/openid-configuration`, 1 h cache), HTTP client with timeout |
| `oidc::sso::states` | Single-use `state`/`nonce`/`code_verifier` in `<key>_oauth_states` (they expire in 10 min; the `state` is kept only as a SHA-256 hash) |
| `oidc::sso::login` | The `IdentityProvider` trait and the whole flow: `discover_options`, `start_login`, `handle_callback` (login and, via `complete_purpose`, the calendar or mailbox connection), `sign_in` |
| `oidc::sso::settings` | Login settings fields (`client_id`, `client_secret`, `domains`, `require_sso`) and validation |
| `oidc::calendar` | Calendar sync engine (per-user connection, jobs, event consumer, mapping and conflicts) used by `google` and `microsoft`, with the per-provider `CalendarApi` trait |
| `oidc::mail` | Base for the OAuth mailbox providers (contributed as `dyn email::MailboxProvider`) of `google` and `microsoft`: the `MailOAuth` trait (scopes and consent parameters), connection flow (`connect::start`/`connect::complete`, `state` with the `mail` purpose, mailbox creation through `email::service::mailboxes::create` with the user's scope), token renewal (`tokens::with_access`: renews based on `expires_at` and, on a 401, once; returns the new credentials to the core), MIME reading with `mail-parser` and building with `lettre` (`mime::parse_remote`/`mime::build`) and `mail_*` errors |

## Configure

1. In the identity provider, create an OIDC application of type *web* (authorization code; PKCE is always sent).
2. **Settings → Integrations → Modules → Single sign-on (OIDC)**: the panel shows the **callback URL** (`{public address}/api/v1/public/oidc/callback`). Register it as a *redirect URI* at the provider.
3. Scopes: `openid email profile`. The provider must send `email` and `email_verified = true` in the id_token.
4. Fill in and enable:

| Setting | Type | Notes |
| --- | --- | --- |
| `discovery_url` | URL (required) | Issuer (`https://empresa.okta.com`) or the full discovery URL. Only `https://` |
| `button_label` | Text | Button label (default "Login corporativo") |
| `client_id` | Text (required) | |
| `client_secret` | Secret (required) | Encrypted in the `SecretVault`; never returned by the API |
| `domains` | Long text | Comma-separated email domains. **Without domains the button does not appear** on the login screen |
| `require_sso` | Boolean | Password login for these domains responds 403 `sso_required` (requires domains) |

## Flow

1. The login screen calls `POST /api/v1/auth/sso/discover` with the email. `discover` checks whether the module is enabled and returns a button when the email's domain matches `domains`, pointing to `/api/v1/public/oidc/login?login_hint=<email>` (`required = require_sso`).
2. `GET …/public/login`: rate limit per IP (30/min), module enabled, `client_id` configured, optional `login_hint` (the typed email; if present, it must be in the configured domain and is passed on to the provider), `redirect` accepted only as a relative path (`/…`, without `//`). It stores the attempt and redirects (302) to the provider with `state`, `nonce`, PKCE and `login_hint`.
3. `GET …/public/callback`: consumes the `state` (single use, 10 min), exchanges the code directly at the token endpoint (TLS), validates `iss` (equal to the discovery one), `aud`/`azp`, `exp`/`iat` (5 min tolerance) and `nonce`. The id_token signature is not verified via JWKS because it comes directly from the token endpoint over TLS (OIDC Core 3.1.3.7). It requires `email_verified`, the configured domain and an active user with that email; then `oidc::sso::session::sign_in` creates the session through the kernel (method `oidc`) and the browser goes to `redirect` (or `/`) — or to `/login/two-factor` when the user has a second factor — with the `Set-Cookie`.
4. Failures go back to `/login?sso_error=<code>`.

The `client_secret` is read from the `SecretVault` with `module_secret_key` (module secret, without user scope), so starting the login does not reveal whether the email has an account: the active user check only happens on the callback, with the email verified by the provider. Whoever signs in is always the owner of that email.

## Routes

| Method | Path | Authentication | What it does |
| --- | --- | --- | --- |
| `GET` | `/api/v1/oidc/status` | Session/token | Configuration summary (without the secret) and callback URL |
| `GET` | `/api/v1/public/oidc/login?login_hint=&redirect=` | Public | Starts the login (302) |
| `GET` | `/api/v1/public/oidc/callback` | Public | Provider callback (302) |

## `sso_error` codes

`no_account` (no active user with that email — the login screen has its own message), `invalid_state`, `invalid_request`, `access_denied`, `sso_disabled`, `sso_not_configured`, `email_not_verified`, `domain_not_allowed`, `token_exchange_failed`, `invalid_grant`, `invalid_token`, `provider_unavailable`, `rate_limited`, `sso_failed`.

## Permissions, events and data

- No permissions or events of its own: configuration is done on the core modules screen (Settings → Integrations → Modules, `settings.manage`) and the session is created by the kernel (`sign_in`, method `oidc`), with the same second-factor requirement as password login.
- `oidc_oauth_states` table (migration 1): attempts in progress; expired rows are deleted on every new start.
- Sent to the provider: `client_id`, `client_secret` (only at the token endpoint), callback URL, the email as `login_hint`. Received: the id_token (read and discarded). **No provider token is stored** or recorded in logs, audit, events or responses.

## Limitations

- No automatic provisioning (JIT/SCIM): the user's account must exist (invitation).
- No single logout (front/back-channel logout).
- Providers that do not send `email_verified` do not work.
