# "Google" module (`google`)

Three features, independent of each other within the same module:

- **Sign in with Google** (PLAN 12.7): a "Continue with Google" button for the configured Google Workspace domains, linking to the existing user by verified email; the administrator can require single sign-on for those domains.
- **Google Calendar sync** (PLAN 5.8): each user connects their own calendar; CRM meetings go to the primary calendar and calendar events with CRM people become meetings in the CRM, in both directions, with "the last change wins".
- **Gmail mailbox** (session 22): `google.gmail` provider for the core's individual email ([email.md](../architecture/email.md)); each user connects their own Gmail in **My account → Email mailboxes**.

The OAuth/OIDC code and the sync engine come from the `sso` module (crate `aphel-mod-sso`, see [sso.md](sso.md)); `google` declares `depends_on("sso")`, so `sso` must be in the build and enabled.

## Configure in the Google Cloud Console

1. Create (or choose) a project and enable the **Google Calendar API** (calendar) and the **Gmail API** (mailboxes).
2. **OAuth consent screen**: type *Internal* (only your Workspace organization) or *External*. Scopes used: `openid`, `email`, `profile` (login), `https://www.googleapis.com/auth/calendar.events` (calendar), `https://www.googleapis.com/auth/gmail.readonly` and `https://www.googleapis.com/auth/gmail.send` (email). The Gmail scopes are **restricted**: with an *External* screen Google requires app verification (and a security assessment) before releasing it beyond the test users; with an *Internal* Workspace screen there is no verification.
3. **Credentials → Create credentials → OAuth client ID → Web application**. Under *Authorized redirect URIs*, register the **callback URL** shown in the module panel: `{public address}/api/v1/public/google/callback` (the same for login, calendar and email).
4. In Aphel, **Settings → Integrations → Modules → Google**: fill in and enable.

| Setting | Type | Notes |
| --- | --- | --- |
| `client_id` | Text (required) | |
| `client_secret` | Secret (required) | Encrypted; never returned by the API |
| `domains` | Long text | Single sign-on domains, comma-separated. Empty = no login button (the calendar keeps working) |
| `require_sso` | Boolean | Password login for these domains responds 403 `sso_required` |
| `meeting_type` | Text | Key of the activity type for synced meetings (default `meeting`) |
| `import_scope` | List | `with_contacts` (default: only events with some attendee who is already a contact) or `all` (all timed events) |

## Login

Same flow as [`sso`](sso.md#flow), with Google's fixed endpoints (`accounts.google.com`), `prompt=select_account`, issuer `https://accounts.google.com` and the `email_verified = true` requirement. Starts at `GET /api/v1/public/google/login?login_hint=&redirect=`; failures go back to `/login?sso_error=<code>` (same codes as `sso`). No login token is stored.

## Calendar

### Connect

- Permission **`google.calendar_sync`** ("Connect and sync your own Google calendar"; administrators already have it).
- **Google** menu (`/google`) → **Connect calendar**: `POST /calendar/connect` returns the consent URL (`access_type=offline`, `prompt=consent`, `login_hint` = the user's email) with a single-use `state` bound to the user. The callback goes through the same callback URL, validates the id_token, stores the encrypted **refresh token** in the `SecretVault` (`user:<id>:refresh_token`) and the connection in `google_calendar_connections` (without tokens), publishes `google.calendar_connected` with audit and enqueues the first sync. The browser goes back to `/google?calendar=connected` (or `?calendar_error=<code>`).
- **Disconnect** (`DELETE /calendar/connection`) deletes the secret and the connection and publishes `google.calendar_disconnected`. The meetings and the mapping remain (reconnecting the same account does not duplicate anything).

### Sync

- The scheduled job `google.calendar_sync_all` runs every 5 minutes and enqueues `google.calendar_sync` per connection (also triggered by **Sync now**).
- **Calendar → CRM**: `events.list` of the primary calendar (`singleEvents=true`, `showDeleted=true`); the first load covers from 30 days ago to 90 days ahead and then uses the incremental `syncToken` (410 → full load again). For each timed event (all-day events are ignored):
  - new and within the window → creates an activity of type `meeting_type`, owner = the user, with the event's title, start, duration and description, status `done` if it has already ended; attendees whose email belongs to an **existing contact** become links of the activity (unknown emails do **not** create people; with `import_scope = with_contacts` an event without a contact is ignored);
  - already linked and changed → updates the title, time, duration, description and adds contact links (existing links remain);
  - cancelled/deleted → deletes the activity (it goes to the core's trash) and the link.
  Changes go through the core's use cases (`/v1/tasks`) with the user's permissions: regular events, audit, automations and notifications.
- **CRM → calendar**: the `google.calendar_export` consumer receives `activity.created|updated|deleted` for meetings (type `meeting_type`) of connected users and schedules the `google.calendar_push` job 15 s later (grouping consecutive edits). It creates/updates (`sendUpdates=none`) or deletes the event in the primary calendar with title, start, end (duration; 30 min if empty) and description. **Attendees are not sent**, so invitations are never sent to customers.
- **Mapping** in `google_calendar_links` (`activity_id`, `remote_id`, `remote_etag`, `remote_updated_at`, `local_updated_at`, `last_synced_at`). Loops are avoided like this: after each write the module stores the resulting local `updated_at`; the export ignores an activity whose `updated_at` is not newer than the stored one (it is the sync itself).
- **Conflict**: if both sides changed since the last sync, it compares Google's `updated` with the CRM's `updated_at` and the most recent change wins (if the CRM won, the change is sent to the calendar again).
- Errors are stored on the connection (`last_error_code`/`last_error_message`, visible in `/google`): `calendar_authorization_revoked` (refresh token revoked/expired → status `revoked`, ask the user to reconnect), `calendar_permission_revoked`, `calendar_member_unavailable`, `calendar_meeting_type_not_found`, `calendar_api_failed`.

## Mailbox (Gmail)

`gmail` provider contributed as `dyn email::MailboxProvider` (key in the core `google.gmail`, `oauth` connection with `start_path = /api/v1/google/mail/connect`). Sync, links, visibility and sending belong to the core; the extension only talks to the Gmail API.

### Connect

- On the **Email mailboxes** page (`/account/mailboxes`), the Gmail button navigates (full page) to `GET /api/v1/google/mail/connect`. It requires the module to be enabled and `client_id`/`client_secret` to be configured; there is no specific permission (the mailbox is personal; the core's mailbox API decides).
- Consent with `openid`, `email`, `gmail.readonly`, `gmail.send`, `access_type=offline`, `prompt=consent` and `login_hint` = the user's email; single-use `state` with the `mail` purpose, bound to the user (PKCE S256).
- The callback goes through the same callback URL (`/public/callback`): it exchanges the code, validates the id_token and creates the mailbox **as the user** (`email::service::mailboxes::create` with their scope) with `{ provider: "google.gmail", settings: {}, credentials: { access_token, refresh_token, expires_at } }`. The core calls the provider's `connect`, which reads `users.getProfile` to find out the address. The browser goes back to `/account/mailboxes?connected=google.gmail` or `?error=<code>` (`sso_not_configured`, `module_disabled`, `access_denied`, `invalid_state`, `mail_no_refresh_token`, `token_exchange_failed`, `invalid_grant`, `invalid_token`, `provider_unavailable`, `mail_connect_failed` or the code returned by the mailbox API).

### Sync (`sync_since`)

- Cursor `{ phase, history_id, page_token }`.
- **First load** (no cursor): stores the current `historyId` from `users.getProfile` and lists `users.messages.list` with `q=after:<since in seconds>` (`since` = the mailbox's days of history), `maxResults = limit` and pagination by `pageToken` (`has_more = true` while there are pages). Spam and trash are left out.
- **Incremental**: `users.history.list` with `startHistoryId`, `historyTypes=messageAdded`, paginated; when done, the cursor moves to the returned `historyId`. Expired `historyId` (404) → new first load from `since` (the core discards repeats by `Message-ID`).
- Each message is downloaded with `users.messages.get?format=raw` (4 in parallel) and the MIME is read with `mail-parser`: headers, addresses, text, HTML, attachments (including inline ones with `Content-ID`), `Auto-Submitted`, `Content-Type` and, in bounces, the original `Message-ID` (`message/rfc822` or `text/rfc822-headers`). Drafts (`DRAFT`) and spam (`SPAM`) are ignored; `SENT` label → `sent_by_account = true`. Without `Date`, `internalDate` is used.

### Sending (`send`)

- The MIME is built with `lettre` (alternative text + HTML, attachments and inline parts, the core's `Message-ID`, `In-Reply-To`/`References`, the core's extra headers, `Bcc` kept so Gmail delivers and strips it) and sent through `users.messages.send` (`raw` in base64url).
- Replies: the `threadId` is looked up with `q=rfc822msgid:<In-Reply-To>` and sent along, so the message joins the same Gmail conversation (Gmail also requires a compatible subject).
- `provider_message_id` = the message id in Gmail.

### Errors and tokens

- Provider error codes (they become `mail_provider_*` in the mailbox API and are kept in the mailbox state): `mail_token_rejected` (401 even after renewing), `mail_authorization_revoked` (refresh token revoked/expired: reconnect), `mail_not_authorized` (no refresh token), `mail_access_denied` (403: permission not granted or API disabled), `mail_api_failed`, `mail_invalid_message` (invalid address when sending); 429/5xx → unavailable (retried by the core).
- Tokens: `access_token`, `refresh_token` and `expires_at` (RFC 3339) are kept only in the mailbox's `SecretVault` (`mailbox`/`<id>`/`credentials`), written by the core. The access token is renewed with the refresh token when less than 2 minutes remain before it expires (or on a 401 response) and the new credentials go back to the core in `test`, `MailSyncPage.credentials` and `SentMail.credentials`. No token goes into logs, audit, events or responses.

## Routes

| Method | Path | Authentication | What it does |
| --- | --- | --- | --- |
| `GET` | `/api/v1/google/status` | Session/token | Login (configured, domains, requirement, callback URL) and the user's calendar connection |
| `POST` | `/api/v1/google/calendar/connect` | `google.calendar_sync` | Consent URL |
| `DELETE` | `/api/v1/google/calendar/connection` | Session/token | Disconnects your own calendar (404 `calendar_not_connected`) |
| `POST` | `/api/v1/google/calendar/sync` | `google.calendar_sync` | Enqueues the sync (202) |
| `GET` | `/api/v1/google/mail/connect` | Session (navigation) | Redirects to the Gmail consent (302) |
| `GET` | `/api/v1/public/google/login` | Public | Starts the login |
| `GET` | `/api/v1/public/google/callback` | Public | Google callback (login, calendar and email) |

## Events, tables and data

- Events: `google.calendar_connected`, `google.calendar_disconnected` (payload `user_id`, `account_email`; never tokens). Audit with `account_email`.
- Calendar connection and remote-link schemas belong to tasks, partitioned by provider key. The provider contributes no SQL schema.
- **Sent to Google**: `client_id`/`client_secret` (token endpoint), the user's email as `login_hint`, and from CRM meetings: title, start, end and description. **Received**: id_token (account email), refresh/access tokens (refresh encrypted; access only in memory) and events from the primary calendar (title, description, time, attendees).

## Limitations

- Email: no push notifications (Pub/Sub `users.watch`); the mailbox is read on the core's sync schedule. Only new messages (`messageAdded`): label changes, reads or deletions in Gmail are not reflected.
- No push notifications (webhooks) from Google in this version: the calendar is read every 5 minutes.
- Only the primary calendar (`primary`); all-day events are ignored; attendees are not exported.
- Recurring CRM tasks are exported occurrence by occurrence (as the occurrences exist in the CRM); recurring Google events are imported per instance.
- Conflict comparison with second precision.

OAuth nonce/PKCE states are owned by SSO in `sso_oauth_states`, partitioned by provider key. Calendar connections and remote links are owned by tasks in `task_calendar_connections` and `task_calendar_links`.
