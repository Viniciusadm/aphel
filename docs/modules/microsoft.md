# "Microsoft" module (`microsoft`)

- **Sign in with Microsoft** (PLAN 12.7): a "Continue with Microsoft" button (Microsoft Entra ID, work accounts) for the configured domains, linking to the existing user by email; the administrator can require single sign-on for those domains.
- **Outlook calendar sync** (PLAN 5.8) via Microsoft Graph (`/me/calendarView/delta` and `/me/events`), in both directions, with "last change wins".
- **Outlook / Microsoft 365 mailbox**: the `microsoft.outlook` provider of the core individual email ([email.md](../architecture/email.md)); each user connects their own mailbox in **My account → Email mailboxes**.

Depends on the `sso` module (crate `aphel-mod-sso`, `depends_on("sso")`, see [sso.md](sso.md)) and uses the same calendar engine as the [`google`](google.md#calendar) module — the sync, mapping, conflict and error behavior is identical; only what differs is described here.

## Configure in Microsoft Entra ID

1. **Entra ID → App registrations → New registration**. Account types: *accounts in this organizational directory only* (recommended) or *accounts in any organizational directory*.
2. **Authentication → Add a platform → Web** with the **redirect URI** shown in the module panel: `{public address}/api/v1/public/microsoft/callback` (the same for login, calendar and email).
3. **Certificates & secrets → New client secret**: copy the **value**.
4. **API permissions → Microsoft Graph → Delegated**: `openid`, `email`, `profile`, `offline_access`, `Calendars.ReadWrite` (calendar) and `Mail.Read`, `Mail.Send`, `User.Read` (email). Grant admin consent if the directory policy requires it.
5. Optional, recommended: **Token configuration → Add optional claim → ID → `xms_edov`** (indicates whether the email domain is verified).
6. In Aphel, **Settings → Integrations → Modules → Microsoft**: fill in and enable.

| Setting | Type | Notes |
| --- | --- | --- |
| `tenant` | Text | Directory ID (GUID) — accepts only that directory —, a verified domain, or `organizations` (default, any work account). `common`/`consumers` are also accepted |
| `client_id` | Text (required) | Application (client) ID |
| `client_secret` | Secret (required) | Encrypted; never returned by the API |
| `domains` | Long text | Single sign-on domains, comma-separated. Empty = no login button |
| `require_sso` | Boolean | Password login for these domains responds 403 `sso_required` |
| `meeting_type` | Text | Activity type for meetings (default `meeting`) |
| `import_scope` | List | `with_contacts` (default) or `all` |

## Login: how the email is considered verified

Microsoft does not send `email_verified`. Decision:

1. If the id_token carries `xms_edov = true`, the `email` claim is used.
2. Otherwise, the `preferred_username` is used (the work account's UPN, whose domain must be verified in the home directory) **only if the domain is in `domains`**. The `email` claim is never used without `xms_edov`, because the `mail` attribute can be edited in the directory.
3. Otherwise: `email_not_verified`.

The issuer must be `https://login.microsoftonline.com/{tid}/v2.0`; with `tenant` = GUID, the `tid` must match (accounts from other directories are refused). Endpoints: `https://login.microsoftonline.com/{tenant}/oauth2/v2.0/{authorize,token}`, `prompt=select_account`. Other rules, routes and `sso_error` codes: the same as in [`sso`](sso.md#flow). Setting `tenant` = directory ID is recommended whenever only your own company uses Aphel.

## Calendar

- Permission **`microsoft.calendar_sync`**; page `/microsoft`.
- Consent with `offline_access` and `Calendars.ReadWrite` (`prompt=consent`); the refresh token is stored encrypted in `user:<id>:refresh_token`. Microsoft rotates the refresh token on every refresh: the new value replaces the previous one in the `SecretVault`.
- Reading: `GET /me/calendarView/delta` over the -30/+90 day window, with `Prefer: outlook.timezone="UTC"` and a text body; the `@odata.deltaLink` is the incremental cursor (only links from `https://graph.microsoft.com/` are followed; 410 → full load). `@removed` or `isCancelled` events delete the linked activity; `isAllDay` is ignored; rooms (`type = resource`) do not count as attendees.
- Writing: `POST/PATCH/DELETE /me/events` with title, text body, start and end in UTC. **Attendees are not sent** (Outlook would send invitations automatically).
- Jobs `tasks.calendar_sync_all` (every 5 min), `tasks.calendar_sync`, `tasks.calendar_push`; consumer `tasks.calendar_export`; events `microsoft.calendar_connected` / `microsoft.calendar_disconnected` (payload `user_id`, `account_email`).

## Mailbox (Outlook)

The `outlook` provider is contributed as `dyn email::MailboxProvider` (core key `microsoft.outlook`, `oauth` connection with `start_path = /api/v1/microsoft/mail/connect`). Sync, links, visibility and sending belong to the core; the extension only talks to Microsoft Graph.

### Connect

- The Outlook button at `/account/mailboxes` navigates (full page) to `GET /api/v1/microsoft/mail/connect`. It requires the module to be enabled and `client_id`/`client_secret` to be configured; there is no dedicated permission.
- Consent with `openid`, `email`, `profile`, `offline_access`, `Mail.Read`, `Mail.Send`, `User.Read` (`prompt=consent`, `login_hint`), a single-use `state` with purpose `mail` and PKCE S256, on the configured `tenant`.
- The callback (`/public/callback`) exchanges the code, validates the id_token and creates the mailbox **as the user** (`email::service::mailboxes::create`) with `{ provider: "microsoft.outlook", settings: {}, credentials: { access_token, refresh_token, expires_at } }`. The provider's `connect` reads `GET /me` (`mail`, or `userPrincipalName`, and `displayName`). The browser returns to `/account/mailboxes?connected=microsoft.outlook` or `?error=<code>` (the same codes as the [`google`](google.md#connect-1) module).

### Sync (`sync_since`)

- Cursor `{ inbox: { link, delta }, sentitems: { link, delta } }`: the stored link is the `@odata.nextLink` (load in progress) or the `@odata.deltaLink` (incremental). Only links from `https://graph.microsoft.com/` are followed.
- Each call reads the **Inbox** and then **Sent Items** with `GET /me/mailFolders/{folder}/messages/delta?$select=id&$filter=receivedDateTime ge <since>` (first load) or the cursor link, with `Prefer: odata.maxpagesize`, up to `limit` messages in total; `has_more = true` while any folder has a `nextLink`. 410 (expired delta state) → a new load of the folder from `since`. `@removed` items are ignored.
- Each message is downloaded as raw MIME with `GET /me/messages/{id}/$value` (4 in parallel) and parsed with `mail-parser` as in Gmail. Sent Items → `sent_by_account = true`.

### Sending (`send`)

- MIME built with `lettre` (as in Gmail, keeping `Bcc`, `In-Reply-To` and `References`) and sent through `POST /me/sendMail` with `Content-Type: text/plain` and the MIME in base64. Graph responds 202 without an id, so `provider_message_id` stays empty and the sent message shows up later through the Sent Items sync (the core matches it by `Message-ID`).
- Graph limit for MIME sending: 4 MB after base64.

### Errors and tokens

- Provider error codes (they become `mail_provider_*` in the mailboxes API and are kept in the mailbox state): `mail_token_rejected` (401 even after refreshing), `mail_authorization_revoked` (refresh token revoked/expired: reconnect), `mail_not_authorized` (no refresh token), `mail_access_denied` (403: permission not granted or API disabled), `mail_api_failed`, `mail_invalid_message` (invalid address when sending); 429/5xx → unavailable (retried by the core).
- Tokens: `access_token`, `refresh_token` and `expires_at` (RFC 3339) live only in the mailbox's `SecretVault` (`mailbox`/`<id>`/`credentials`), written by the core. The access token is refreshed with the refresh token when less than 2 minutes remain before it expires (or on a 401 response) and the new credentials go back to the core in `test`, `MailSyncPage.credentials` and `SentMail.credentials`. No token goes to logs, audit, events or responses.
- Microsoft rotates the refresh token on every refresh: the new value goes back to the core together with the access token.

## Routes

| Method | Path | Authentication | What it does |
| --- | --- | --- | --- |
| `GET` | `/api/v1/microsoft/status` | Session/token | The user's login and calendar connection |
| `POST` | `/api/v1/microsoft/calendar/connect` | `microsoft.calendar_sync` | Consent URL |
| `DELETE` | `/api/v1/microsoft/calendar/connection` | Session/token | Disconnects the user's own calendar |
| `POST` | `/api/v1/microsoft/calendar/sync` | `microsoft.calendar_sync` | Enqueues the sync (202) |
| `GET` | `/api/v1/microsoft/mail/connect` | Session (navigation) | Redirects to the Outlook consent (302) |
| `GET` | `/api/v1/public/microsoft/login` | Public | Login start |
| `GET` | `/api/v1/public/microsoft/callback` | Public | Microsoft callback (login, calendar and email) |

## Tables and data

- Calendar connection and remote-link schemas belong to tasks, partitioned by provider key. The provider contributes no SQL schema.
- **Sent to Microsoft**: `client_id`/`client_secret` (token endpoint), the user's email as `login_hint`, and from CRM meetings: title, start, end and description. **Received**: id_token, tokens (refresh encrypted; access only in memory) and events from the default calendar.
- No token or secret goes to logs, audit, events or responses.

## Limitations

- Email: no Graph notification subscriptions; the mailbox is read on the core's sync schedule. Only Inbox and Sent Items (subfolders and rules that move messages to other folders are left out). The delta also returns changed messages (read, flagged), which are downloaded again and discarded by the core as duplicates.
- No Graph notification subscriptions (webhooks) in this version: reading every 5 minutes.
- Only the user's default calendar; no personal accounts unless `tenant = consumers`/`common`.
- Same recurrence and precision limitations as the [`google`](google.md#limitations) module.

OAuth nonce/PKCE states are owned by SSO in `sso_oauth_states`, partitioned by provider key. Calendar connections and remote links are owned by tasks in `task_calendar_connections` and `task_calendar_links`.

Tasks owns calendar workers and resolves enabled provider contributions for each job and event. Provider identity travels in the job payload; disabled or removed adapters are skipped.
