---
title: Security
description: Two-factor policy, sessions, API tokens, single sign-on, rate limits, CSRF protection, the audit log, secret handling and privacy requests.
sidebar:
  order: 9
---

This page lists what Aphel protects by default, the controls you can tune, and the practices that fall to you as the operator.

## Checklist for production

1. Serve Aphel over HTTPS and set the public address to the `https://` address (Settings, General). This marks the session cookie `Secure`.
2. Set `APP_ENCRYPTION_KEY` to a random key, keep a copy outside the database backup, and never change it without rotating (see below).
3. Set `HTTP_TRUSTED_PROXIES` if you run behind a reverse proxy.
4. Require two-factor authentication for administrators at least.
5. Leave `WEBHOOKS_ALLOW_PRIVATE_NETWORKS` and `EMAIL_ALLOW_PRIVATE_NETWORKS` off unless you need them.
6. Set `METRICS_TOKEN` only if you scrape metrics, and use a long random value.
7. Back up the database, the files and the `.env`, and test the restore. See [Backup and restore](/docs/admin/backup-and-restore/).

## Passwords and sign-in

- Passwords have 8 to 128 characters and are stored as Argon2 hashes.
- After repeated failures an account is locked. By default, 5 failed sign-ins within 15 minutes lock it until 15 minutes after the latest failure. Tune with `LOGIN_LOCKOUT_ATTEMPTS` and `LOGIN_LOCKOUT_WINDOW_SECS`; `0` in either disables the lockout.
- Links expire: email verification after 48 hours, password reset after 1 hour, invitations after 7 days.
- Changing a password signs out every other session and emails a notice.

## Two-factor authentication

Members can enable time-based one-time passwords (TOTP) under My account, Account security. Activation shows a QR code and, after the first valid code, 10 single-use recovery codes. The secret is stored encrypted, so the feature needs `APP_ENCRYPTION_KEY`.

At sign-in, a member with two-factor enabled gets a partial session that only accepts the code challenge for 10 minutes. Five wrong codes end that session. Each TOTP code works once.

### Organization policy

Administrators with `settings.manage` set who must use two-factor authentication:

| Policy | Applies to |
| --- | --- |
| `none` (default) | Nobody is forced |
| `admins` | Members with the `admin` role |
| `everyone` | All members |

Members covered by the policy who have not enabled it can reach only their security settings until they do (`403 two_factor_setup_required`). API tokens are not affected. You cannot set a policy that covers you while you have no second factor yourself. The API is `GET` and `PUT /api/v1/organization/two-factor-policy`. Policy changes are audited.

## Sessions

- A session lasts 30 days by default. The cookie is `aphel_session`, `HttpOnly`, `SameSite=Lax`, and `Secure` when the public address uses `https`.
- Members see their open sessions, with device and address, under Account security, and can sign out any of them or all the others. `POST /api/v1/auth/logout-all` signs out everywhere.
- Suspending or removing a member ends their sessions.
- If you restore a backup, sessions that existed at backup time become valid again. After a security incident, ask users to sign out of all devices or change their passwords.

## API tokens

Personal API tokens let integrations call the API as a user, narrowed by scopes. Details are in [Roles and permissions](/docs/admin/roles-and-permissions/#api-token-scopes).

- The secret is shown once, when the token is created, and cannot be shown again. Store it safely.
- Tokens can have an expiry and can be revoked at any time. After revocation, requests answer `401 invalid_api_token`.
- Every token is limited by a sliding window set in Settings, API tokens (default 600 requests per 60 seconds). Responses carry `X-RateLimit-Limit`, `X-RateLimit-Remaining` and `X-RateLimit-Reset`, and an excess answers `429`.
- Users with `users.manage` can list and revoke every token. Token creation and revocation are audited and recorded as security events.

## Single sign-on

Sign in with Google, Microsoft or any OpenID Connect provider is configured per module. See [Integrations](/docs/admin/integrations/). Each can list the email domains that use it and refuse password sign-in for those domains. Make sure at least one administrator can still sign in with a password before enforcing it.

## Rate limits

Sign-in related routes have fixed limits shared across processes through the database (with an in-process fallback):

| Route group | Limit |
| --- | --- |
| Sign in | 10 per 5 minutes |
| Password reset | 5 per 15 minutes |
| Two-factor challenge | 10 per 15 minutes |
| Single sign-on | 30 per 5 minutes |
| Password change | 5 per 15 minutes |
| Backup restore in the installer | 5 per hour per address, 20 per hour in total |

Public routes of some modules (forms, booking, quotes, messaging and webhook receivers) have their own limits. Behind a proxy, set `HTTP_TRUSTED_PROXIES` so limits apply per client and not to the proxy.

## Request protection (CSRF)

Browser sessions use a cookie, so Aphel requires the header `X-Requested-With: aphel` on every request that changes data (any method other than `GET`, `HEAD` and `OPTIONS`). Without it the answer is `403 csrf_header_missing`. The header is not required for:

- requests that carry an API token in `Authorization: Bearer`,
- public routes under `/api/v1/public/` and incoming webhooks under `/api/v1/hooks/`.

Clients that use API tokens therefore do not need it. Scripts using a session cookie must send it. See [Authentication](/docs/reference/authentication/).

:::note
The server answers cross-origin requests permissively (CORS). The cookie is `SameSite=Lax` and the header requirement is what protects state-changing requests made with a session.
:::

## Audit log

Changes to records, settings, roles, members and other platform objects write an audit entry: who (a user, an API token, an automation, a plugin or the system), what, which record and the field changes. Secrets appear as `[redacted]`, and fields hidden from a role appear as `[hidden]` in its diffs.

- Read it under Settings, Audit log, or at `GET /api/v1/audit-logs`. It needs the `audit.read` permission. You can filter by record type, record ID, actor and period.
- Security events for an account (sign-ins, failures, lockouts, two-factor changes, password changes, session revocations, API token changes and permission changes) are under My account, Security events. Users with `users.manage` can read the events of the whole installation at `GET /api/v1/security/events/organization`.
- Upgrades are recorded in the `system_upgrades` table.

## Secrets

- Secrets entered in the app (SMTP password, S3 secret key, module credentials, two-factor secrets) are encrypted with AES-256-GCM using `APP_ENCRYPTION_KEY` and stored in the database. The API never returns them, only whether they are configured.
- `APP_ENCRYPTION_KEY` and `DATABASE_URL` live in the environment. Protect the `.env` or `/etc/aphel/aphel.env` file with restrictive permissions. It is not part of the backup scripts, so store a copy safely: without the key, restored secrets cannot be decrypted.
- `aphel doctor` redacts secret values before printing, which makes its report safe to attach to an issue.

### Rotate the encryption key

1. Generate a new key: `aphel secrets generate-key`.
2. Put the old key in `APP_ENCRYPTION_PREVIOUS_KEYS` (comma separated if there are several) and the new key in `APP_ENCRYPTION_KEY`.
3. Restart the server and worker.
4. Re-encrypt everything with the new key:

   ```bash
   aphel secrets rotate
   ```

5. After the command finishes, you can remove the old key from `APP_ENCRYPTION_PREVIOUS_KEYS`.

## Privacy requests

The privacy module (permission `privacy.manage`) handles data subject requests under Settings, Privacy.

- **Search** finds where a person appears: records with matching personal fields, notes, attachments, tasks and import rows. The search term travels in the request body so it never reaches URLs or access logs, and the search is audited.
- **Create a request** of the kind export, anonymize or delete. It starts as `pending` with the selected records. Only a SHA-256 hash and a masked label of the term are stored. The due date defaults to 15 days.
- **Execute** moves it to `processing` and runs it on the worker. Export writes a zip with `dados.json` and the attachments. Anonymize replaces personal fields with neutral values and removes notes and attachments while keeping deals, amounts, stages and dates. Delete removes the records permanently. Anonymize and delete also redact personal values from the history.
- The export file is removed 30 days after the request finishes, and each download is audited.

Fields can be marked as personal data, which drives these operations and the AI option to withhold personal data.

## Reporting vulnerabilities

Follow `SECURITY.md` in the repository to report a vulnerability privately.
