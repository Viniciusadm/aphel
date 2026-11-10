---
title: Authentication
description: How requests are authenticated in Aphel, with browser sessions, API tokens and scopes, the CSRF header, two-factor authentication, single sign-on, rate limits and public endpoints.
sidebar:
  order: 2
---

Aphel authenticates a request in one of two ways: an opaque **session cookie** (used by the web interface) or an **API token** sent as a Bearer credential (used by integrations). Both end up as the same access scope, so a route behaves the same way regardless of how you authenticated, except for the routes that are restricted to browser sessions.

| | Browser session | API token |
| --- | --- | --- |
| Credential | Cookie `aphel_session` | `Authorization: Bearer aphel_<prefix>_<secret>` |
| Obtained with | `POST /api/v1/auth/login` | `POST /api/v1/api-tokens` (needs a session) |
| CSRF header `X-Requested-With: aphel` | Required on requests that change data | Not required |
| Permissions | The role of the user | Scopes of the token, intersected with the permissions of its creator at the time of each request |
| Rate limit | Per endpoint, for sign-in and similar actions | Per token, sliding window |
| Account routes (`/auth/*`, `/api-tokens`, `/two-factor/*`, `/security/*`) | Accepted | Refused with `403 api_token_not_allowed` |

## Session cookie

`POST /api/v1/auth/login` with the email and password of an active user starts a session and answers `200` with the current user, organization, role, teams and effective permissions (the same payload as `GET /api/v1/auth/me`).

```bash
curl -i -X POST "$APHEL_URL/api/v1/auth/login" \
  -H "Content-Type: application/json" \
  -H "X-Requested-With: aphel" \
  -c cookies.txt \
  -d '{"email":"ana@example.com","password":"correct horse battery staple"}'
```

The session cookie is set as:

```text
aphel_session=<opaque token>; Path=/; HttpOnly; SameSite=Lax; Max-Age=<seconds>; Secure
```

- `HttpOnly` keeps the token away from page scripts and `SameSite=Lax` keeps it out of cross-site `POST` requests.
- `Secure` is added when the installation is configured for secure cookies, which follows the saved public address of the organization (`https`).
- The token is random and opaque. Only a hash of it is stored in the database, so a session can be revoked at any time.
- A session lasts 30 days from sign-in. Requests refresh the last-access time at most once a minute.
- `POST /api/v1/auth/logout` revokes the current session and clears the cookie; `POST /api/v1/auth/logout-all` revokes every session of the user. Users can list and revoke sessions in `GET /api/v1/security/sessions`, `DELETE /api/v1/security/sessions/{session_id}` and `POST /api/v1/security/sessions/revoke-others`. Changing the password ends every other session.
- Invited and deactivated users cannot authenticate. A user whose organization requires two-factor authentication, but who has not enabled it, can only reach their security settings (`403 two_factor_setup_required`).

Other public account actions under `/api/v1/auth/`: `verify-email`, `resend-verification`, `forgot-password`, `reset-password`, `invitation`, `accept-invitation` and `sso/discover`. Links sent by email expire after 48 hours (email verification), 1 hour (password reset) or 7 days (invitations), and each token works once.

## CSRF header

Requests that change data (any method other than `GET`, `HEAD` and `OPTIONS`) must carry:

```text
X-Requested-With: aphel
```

The check is case-insensitive on the value. A request without it is refused with `403 csrf_header_missing`. Browsers cannot add this header to cross-site form submissions, which is what protects the session cookie.

The rule has three exceptions:

| Exception | Why |
| --- | --- |
| The request has an `Authorization: Bearer ...` header | There is no ambient credential to abuse. |
| The path starts with `/api/v1/public/` | Public pages and callbacks (booking, quotes, forms, sign-in callbacks, plugin public routes). |
| The path starts with `/api/v1/hooks/` | Incoming webhooks authenticate with the secret in the URL. |

Everything else, including `POST /api/v1/auth/login`, needs the header.

## API tokens

### Format

A token looks like this:

```text
aphel_3f9a1c2b_9c1e0d7a4b2f6e8d0a1b3c5d7e9f1a2b4c6d8e0f1a3b5c7d9e1f2a4b6c8d0e2f4a
```

It has the literal prefix `aphel_`, an 8-character hexadecimal identifier and a hexadecimal secret of at least 32 characters, separated by underscores. The first two parts (for example `aphel_3f9a1c2b`) are shown in listings as `token_prefix` so a token can be recognised; only a hash of the whole token is stored, and the full value is returned once, when it is created.

### Creating and revoking

Creating a token needs a browser session, because token routes refuse API tokens. In the interface use **Settings, API tokens**, or call the API:

```bash
curl -X POST "$APHEL_URL/api/v1/api-tokens" \
  -b cookies.txt \
  -H "Content-Type: application/json" \
  -H "X-Requested-With: aphel" \
  -d '{"name":"Website integration","scopes":["people:read","people:write","deals:write"],"expires_at":"2027-01-31T23:59:59Z"}'
```

The `201` response contains the token metadata plus `secret`, the only time the full token is visible:

```json
{
  "data": {
    "id": "0192a1b2-0000-7000-8000-000000000001",
    "name": "Website integration",
    "token_prefix": "aphel_3f9a1c2b",
    "scopes": ["deals:write", "people:read", "people:write"],
    "created_by": "0192a1b2-0000-7000-8000-0000000000aa",
    "created_by_name": "Ana Souza",
    "expires_at": "2027-01-31T23:59:59Z",
    "last_used_at": null,
    "last_used_ip": null,
    "revoked_at": null,
    "created_at": "2026-10-20T13:45:00Z",
    "secret": "aphel_3f9a1c2b_9c1e0d7a4b2f6e8d0a1b3c5d7e9f1a2b4c6d8e0f1a3b5c7d9e1f2a4b6c8d0e2f4a"
  }
}
```

| Route | Purpose |
| --- | --- |
| `GET /api/v1/api-tokens` | Lists your tokens. `?owner=all` lists everyone's tokens and needs `users.manage`. Active tokens come first. The secret is never returned again. |
| `POST /api/v1/api-tokens` | Creates a token (`name`, `scopes`, optional `expires_at`). |
| `GET /api/v1/api-tokens/scopes` | The scopes you may grant, derived from your own permissions. |
| `POST /api/v1/api-tokens/{token_id}/revoke` | Revokes a token. Members revoke their own; `users.manage` revokes any. Revoking twice is harmless. |

Rules applied on creation:

- `name` has 1 to 100 characters.
- `scopes` is a non-empty list of at most 100 known scopes, and you can only grant scopes backed by permissions you currently have (`422 validation_error` on `scopes` otherwise).
- `expires_at`, when present, must be in the future. A token without it does not expire.
- A user can have at most 50 active tokens (`422 too_many_api_tokens`).
- Creating and revoking tokens is recorded in the audit log and the security event log, and publishes `api_token.created` and `api_token.revoked`.

### Scopes

A scope has the form `<resource>:read` or `<resource>:write`. It is derived from permission keys: a permission `resource.read` becomes `resource:read`, and every other action (`create`, `update`, `delete`, `export`, `archive`, `manage`, and so on) becomes `resource:write`. For example `people.read` gives `people:read`, and `people.create`, `people.update`, `people.delete` and `people.export` all give `people:write`.

Scopes are therefore coarser than permissions: a token with `people:write` is allowed to try every non-read action on people, but each one is still checked against the permissions of its creator. Custom objects use the resource `custom.<key>`, so the scopes are `custom.<key>:read` and `custom.<key>:write`. The [permissions catalog](/_aphel/docs/reference/permissions-catalog/) lists the permission keys; `GET /api/v1/api-tokens/scopes` lists the scopes available to you.

### Effective permissions

On every request with a token, Aphel loads the creator's current role and permissions and keeps only those whose scope appears in the token:

```text
effective permissions = permissions of the creator now  ∩  scopes of the token
```

Consequences:

- A token can never do more than its creator can do at that moment. If the creator loses a permission, or changes role, the token loses it immediately.
- The record scope (own, team or organization) of the creator still applies.
- A permission missing because the token lacks the scope answers `403 insufficient_scope`. A permission missing because the creator does not have it answers `403 permission_denied`.
- A token that is expired, revoked, malformed or whose creator is deactivated answers `401 invalid_api_token`.
- Actions taken through a token are attributed to the token's actor in events and the audit log.

### Limits

| Limit | Value |
| --- | --- |
| Active tokens per user | 50 |
| Scopes per token | 100 |
| Requests per token | 600 per 60 seconds by default; administrators can set 1 to 100000 requests per 1 to 3600 seconds |

Every response to a token request carries `X-RateLimit-Limit`, `X-RateLimit-Remaining` and `X-RateLimit-Reset` (seconds until the window frees up). When the limit is exceeded the answer is `429 rate_limited` with `Retry-After` and `details.retry_after` (seconds) and `details.limit`.

## Two-factor authentication

Users can protect their account with a TOTP authenticator app. The flow interacts with sign-in as follows:

1. `POST /api/v1/auth/login` with valid credentials, for an account with two-factor enabled, answers `200 { "two_factor_required": true, "expires_at": "..." }` and sets a **partial session** cookie.
2. A partial session only accepts `POST /api/v1/auth/two-factor/challenge` and logout, for 10 minutes.
3. The challenge takes `{ "code": "123456" }`, either a 6-digit TOTP code (a 30-second step of tolerance either side, each code works once) or an unused recovery code. On success the partial session is replaced by a full session with a new cookie.
4. After five wrong codes the partial session is revoked (`401 two_factor_attempts_exceeded`) and the user must sign in again. A wrong code answers `422 invalid_two_factor_code`.

Accepting an invitation for an account with two-factor authentication follows the same two steps.

| Route | Purpose |
| --- | --- |
| `GET /api/v1/two-factor` | State of the current user. |
| `POST /api/v1/two-factor/setup` | Generates a new secret and returns the `otpauth://` URI and an SVG QR code. Nothing is enabled yet. |
| `POST /api/v1/two-factor/confirm` | Enables two-factor authentication with a code from the app and returns 10 single-use recovery codes, shown only once. |
| `POST /api/v1/two-factor/recovery-codes` | Replaces the recovery codes with 10 new ones, after confirming with the password or a code. |
| `POST /api/v1/two-factor/disable` | Disables it; needs the current password and a TOTP or recovery code. |
| `GET` and `PUT /api/v1/organization/two-factor-policy` | Who must use it: `none`, `admins` (the system administrator role) or `everyone`. Needs `settings.manage`. |

When the policy covers a member who has not enabled two-factor authentication, that member can only reach their security settings until they do (`403 two_factor_setup_required`). API tokens are not affected by the policy. All of these routes are session-only.

## Single sign-on

When single sign-on modules are enabled (the generic OpenID Connect module `sso`, plus the `google` and `microsoft` modules), the web interface starts a provider flow from `/api/v1/public/<provider>/login` and finishes it at `/api/v1/public/<provider>/callback`, which sets the normal session cookie.

- `POST /api/v1/auth/sso/discover` takes `{ "email": "..." }` and returns `data[]` options with `provider`, `label`, `url` and `required`. The list can be empty.
- When an email domain requires single sign-on, password login for it is refused with `403 sso_required`.
- `GET /api/v1/sso/status` shows the single sign-on configuration summary (never the client secret) and the callback URL to register at the identity provider.
- A signed-in provider user that is not an active member of the organization is refused with `403 not_a_member`.

SSO accounts get the same session cookie, so everything above about sessions and the CSRF header applies. API tokens are always created from a session.

## Rate limits and lockout

| Action | Limit | Answer when exceeded |
| --- | --- | --- |
| Sign-in | 10 attempts per 5 minutes for an IP and email pair | `429 rate_limited` |
| Repeated sign-in failures | The account is locked for that IP | `429 login_locked` with `details.retry_after` |
| SSO lookup (`auth/sso/discover`) | 10 lookups per 5 minutes per IP | `429 rate_limited` |
| Forgot password | 5 requests per 15 minutes per IP | `429 rate_limited` |
| Resend verification | 5 requests per 15 minutes per user | `429 rate_limited` |
| Password change | Limited per user | `429 rate_limited` |
| Two-factor challenge | 5 wrong codes per partial session | `401 two_factor_attempts_exceeded` |
| API token requests | 600 per 60 seconds by default, per token | `429 rate_limited` |
| Incoming webhook URL | 120 requests per minute per webhook | `429 rate_limited` |

Every `429` carries `Retry-After` in seconds. Limits are shared between server processes when the database is available and fall back to a per-process counter otherwise.

## Public endpoints

These routes need no credential and no CSRF header. Each has its own protection.

| Path | Purpose | How it is protected |
| --- | --- | --- |
| `/api/v1/auth/login`, `forgot-password`, `reset-password`, `verify-email`, `invitation`, `accept-invitation`, `sso/discover` | Account entry points | Need the CSRF header; rate limited; single-use tokens |
| `/api/v1/installer*` | First-run wizard and backup restore | Available only before the installation is complete |
| `/api/v1/hooks/{token}` | Incoming webhooks | Secret token in the URL and optional HMAC signature, see [Webhooks and events](/_aphel/docs/reference/webhooks-and-events/#incoming-webhooks) |
| `/api/v1/public/book/*` | Public booking pages | Signed management links for changes |
| `/api/v1/public/quotes/{token}` | Public quote page, accept and reject | Signed link |
| `/api/v1/public/forms/*` | Lead forms | Allowed-origin checks and a 64 KB body limit |
| `/api/v1/public/messaging/whatsapp/{channel_id}` | WhatsApp Cloud API callbacks | Verify token and signature |
| `/api/v1/public/esign/webhook` | Electronic signature provider callbacks | Signature with the configured webhook secret |
| `/api/v1/public/google/*`, `/api/v1/public/microsoft/*`, `/api/v1/public/sso/*` | Sign-in flows | Provider state parameters |
| `/api/v1/public/themes/*`, `/api/v1/public/wasm-runtime` | Static design assets and the list of enabled plugin web bundles | Read-only, no secrets |
| `/api/v1/public/p/{key}/*` | Public routes of a runtime plugin that declares them | Rate limited; run as the plugin's installer |

See [Security](/_aphel/docs/admin/security/) for the administrator view of sessions, tokens and two-factor policy.
