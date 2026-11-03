# "Messaging channels" module (`messaging`)

Shared inbox (PLAN 10.5 and 10.6) with two channels: **WhatsApp Cloud API** (Meta's official API, the reference connector) and **IMAP/SMTP email**. Received and sent messages become conversations per channel, with assignment, status (open, pending, resolved), links to a contact and a deal, and replies from within the CRM itself; each message shows up on the contact's timeline (and the deal's, if linked). There are no SLAs, queues or help desk rules (PLAN section 25: outside the core).

Code: `back/modules/messaging` (crate `aphel-mod-messaging`, `MessagingModule`) and `web/src/modules/messaging`.

## For users

1. **Settings → Integrations → Modules → Messaging channels → Enable.** Module settings:
   - **Create a contact for unknown senders** (`create_unknown_contacts`, on by default): when a message arrives from a phone number or email address without a contact, the contact is created (the WhatsApp profile name or the email sender's name, and the phone in `mobile`/`phone` or the email address). When off, the conversation stays without a contact until someone links one.
   - **Acting user** (`acting_user_email`, optional): see [On whose behalf](#on-whose-behalf).
2. Permissions (administrators already have all of them):
   - `messaging.view`: see the inbox, the conversations and the **Conversations** tab of people and deals.
   - `messaging.reply`: reply, assign, change status and link contact/deal.
   - `messaging.manage_channels`: create, edit, test and delete channels and credentials.
3. **Inbox → Channels** (`/messaging/channels`) → **WhatsApp** or **E-mail**, fill in and save. **Test** checks the credentials and shows the result; the channel status becomes "Working" or "Error" with the last error message.
4. **Inbox** (`/messaging`): list with filters (status, mine, unassigned, channel, unread, search) and the conversation panel with the messages, the reply composer (with tools from other modules, context `message` or `email`), status, assignee and links. On narrow screens the list and the conversation alternate.
5. On the contact and deal pages, the **Conversations** tab lists the linked conversations; the timeline shows "Message received", "Reply sent" and "Failed to send" with an excerpt and a link to the conversation.

### WhatsApp Cloud API: setup at Meta

1. In [Meta for Developers](https://developers.facebook.com/), create an app of type **Business** and add the **WhatsApp** product. Link (or create) the WhatsApp Business account and register the number.
2. In **WhatsApp → API Setup**, copy the **Phone number ID** (the number's numeric identifier, not the phone number) and, optionally, the **WhatsApp Business Account ID**.
3. Generate a **permanent token**: Business Settings → System users → create a system user with access to the app and the WhatsApp account and generate a token with the `whatsapp_business_messaging` permission (and `whatsapp_business_management` for the connection test). The temporary token from the dashboard expires in 24 h.
4. In **App settings → Basic**, copy the **App secret**.
5. In Aphel, create the WhatsApp channel with the Phone number ID, access token, app secret and a **verify token** (a random text you make up). Save.
6. The channel screen shows the **webhook URL** (`https://<your-aphel>/api/v1/public/messaging/whatsapp/<channel-id>`). In Meta's **WhatsApp → Configuration** (Webhooks), enter that URL as the *Callback URL* and the same verify token in *Verify token*; click verify and **subscribe to the `messages` field**.
7. Aphel's public address (Settings → General) must be HTTPS (Meta does not call `localhost`; in development use a tunnel).

How the webhook works:

- `GET` answers Meta's verification: checks `hub.mode=subscribe` and `hub.verify_token` (constant-time comparison) and returns `hub.challenge` as text.
- `POST` reads the raw body (up to 1 MB), checks `X-Hub-Signature-256` (HMAC-SHA256 of the body with the app secret, constant-time comparison) and only then parses the JSON. It requires the module to be enabled and is limited to 1200 requests per minute per channel (verification: 30 per minute per IP). Messages for an inactive channel are accepted (200) and ignored. Payloads for another `phone_number_id` are ignored.
- Each new message becomes a `messaging.inbound` job (up to 8 attempts); processing is idempotent by the message id at Meta (`UNIQUE (channel_id, external_id)`), so Meta redeliveries do not duplicate anything.
- Delivery statuses (`sent`, `delivered`, `read`, `failed`) update the sent message (they only move forward; `failed` stores Meta's error code and title).
- Text, buttons, interactive replies, media captions, reactions and location become text; media (image, audio, video, document, sticker) shows up as "[type]" with the caption, without downloading the file.

**24-hour window:** WhatsApp only accepts free-form text up to 24 h after the customer's last message. Outside the window the reply is refused with 422 `whatsapp_window_closed` (the composer warns beforehand); if Meta refuses it anyway (code 131047), the message gets the `failed` status and the same code. Template messages are not supported yet: to resume the conversation, the customer has to write first (or use Meta's tool).

### Email (IMAP/SMTP)

The channel has no IMAP/SMTP stack of its own: it uses the `imap` provider from the `email` module (`email::store::mail_providers()`, the same one used by individual mailboxes), building a mailbox account from the channel's configuration and credentials. That is why the same network and size rules of the `email` module apply.

- Fields: address, sender name, IMAP server/port/user (implicit TLS, default 993; empty user = address) and SMTP (port 465 with TLS or 587 with STARTTLS; empty user = IMAP user). Credentials: IMAP password (required) and SMTP password (optional; empty uses the IMAP password). For Gmail and Microsoft 365 use an app password (or an account with basic IMAP/SMTP enabled).
- Network: servers that resolve to private, loopback or link-local addresses are refused, unless `EMAIL_ALLOW_PRIVATE_NETWORKS=true` (the same variable as the `email` module; useful in development with Mailpit or an internal server).
- **Test** opens the IMAP session (examines the INBOX) and checks SMTP; the result is just "IMAP and SMTP connected" (without an INBOX message count).
- Receiving: the `messaging.poll` schedule runs every 2 minutes and creates one `messaging.poll_channel` job per active mailbox (unique per channel). It reads only the **INBOX** (the provider cursor is built without a sent folder), only messages with a UID greater than the last one seen (stored on the channel, `imap_uid_validity`/`imap_last_uid`), up to 25 per round, without marking them as read. On the first read (or if the server changes `UIDVALIDITY`, or if the IMAP account changes) it starts from the messages that arrive from then on: earlier history is not imported.
- Messages above the provider limit (`EMAIL_SYNC_MAX_MESSAGE_MB`, default 25 MB) come in with headers only (type `too_large`). The body is stored as **plain text** (the text part, or the HTML converted to text, up to 100 thousand characters); HTML, images and attachments are not stored. Messages sent by the channel's own address are ignored.
- Conversations by **thread**: `In-Reply-To`/`References` that match a channel message go into the same conversation; otherwise a new conversation is opened (with the subject).
- Sending goes through the provider's SMTP with its own `Message-ID`, `In-Reply-To` and `References` from the last received message and the subject "Re: …". After sending, a copy is saved via IMAP `APPEND` to the account's sent folder (detected by the `\Sent` attribute or by the usual names; servers that already store the copy themselves, such as Gmail and Microsoft 365, are skipped). An `APPEND` failure only logs a warning; the message is still sent.

### On whose behalf

Webhooks and jobs have no signed-in user, so the module builds a user's scope (`actor::scope_for_user`, on top of `build_access_scope`) to act. The module acts, in this order, as: the **channel's acting user** (a channel field), the **module's acting user** (`acting_user_email`) or **whoever created the channel**, as long as they are an active member. That user must be able to read people (to find the sender) and create people (to create unknown ones); the events and audit entries of received messages are recorded on their behalf. Webhook verification and signature checks use the channel secrets read from the `SecretVault` without a user, and processing runs in the `inbound` job. Without any valid member, email reading records the error on the channel and messages already accepted are stored without contact, event or audit entry.

Replies are sent with the scope of whoever replied (or the acting user, if that person was deactivated).

## What goes to third parties

- **Meta (WhatsApp Cloud API, `graph.facebook.com/v21.0`)**: the recipient's phone number and the reply text; the access token in the `Authorization` header. The connection test reads the number (verified name and display phone number).
- **Configured IMAP/SMTP servers**: user name and password (only inside TLS), the reply email (sender, recipient, subject, text, thread headers), which is also saved to the account's sent folder.
- Nothing else leaves Aphel: contact, deal and other modules' data are not sent.

## Secrets

Per-channel credentials are kept in the `SecretVault` (`ctx.secrets()`, `owner_type = "module"`, `owner_id = "messaging"`, name `channel:<id>:<credential>`: `access_token`, `app_secret`, `verify_token`, `imap_password`, `smtp_password`), encrypted with `APP_ENCRYPTION_KEY` and written in the same transaction as the channel. They are never returned by the API (`secrets` only shows `true`/`false` per credential), nor in events, audit entries (they appear as `secrets.<name>` with `"[redacted]"`), logs or error messages (provider errors store only the provider's code and text, without headers or URLs with credentials). The non-secret configuration (`config`) is JSON in the channel table. Deleting the channel or removing the module data removes the credentials.

## Routes

Authenticated (`/api/v1/messaging`):

| Method | Path | Permission | What it does |
| --- | --- | --- | --- |
| `GET` | `/channels` | `messaging.view` or `messaging.manage_channels` | Channels; `config`, `secrets`, `last_error` and `webhook_url` only for those who manage them |
| `POST` | `/channels` | `messaging.manage_channels` | Creates (`kind`, `name`, `config`, `secrets`, `is_active`, `acting_user_id`); event `messaging_channel.created` |
| `GET` | `/channels/{id}` | `messaging.manage_channels` | Detail |
| `PUT` | `/channels/{id}` | `messaging.manage_channels` | Updates; an empty credential keeps the current one, `clear_secrets` removes optional ones; event `messaging_channel.updated` |
| `DELETE` | `/channels/{id}` | `messaging.manage_channels` | Deletes a channel without conversations and its credentials (with conversations: 409, deactivate it); event `messaging_channel.deleted` |
| `POST` | `/channels/{id}/test` | `messaging.manage_channels` | Tests the connection (20 per minute per installation) and updates the status |
| `GET` | `/conversations?channel_id=&status=&assignee=me\|none\|any\|<id>&contact_id=&deal_id=&unread=&search=&page=&per_page=` | `messaging.view` | Paginated list, most recent activity first |
| `GET` | `/conversations/{id}` | `messaging.view` | Conversation with the last 300 messages |
| `POST` | `/conversations/{id}/messages` | `messaging.reply` | Reply `{ body }` → 202 with the `queued` message; job `messaging.send`; event `message.sent` |
| `PUT` | `/conversations/{id}/assignee` | `messaging.reply` | `{ user_id }` (`null` unassigns; the user needs `messaging.view`); event `conversation.assigned` |
| `PUT` | `/conversations/{id}/status` | `messaging.reply` | `{ status: open\|pending\|resolved }`; event `conversation.status_changed` |
| `PUT` | `/conversations/{id}/links` | `messaging.reply` | `{ contact_id, deal_id }` (`null` removes); event `conversation.linked` |
| `POST` | `/conversations/{id}/read` | `messaging.view` | Resets the unread counter |

Public (`/api/v1/public/messaging`, no authentication): `GET` and `POST` `/whatsapp/{channel_id}` (Meta webhook, above).

Visibility: anyone with `messaging.view` sees all conversations (shared inbox). The linked contact and deal only show up with a name for those who can read the record (`RecordTypeHandler::load` with the user's scope); for others they come as `{ id, accessible: false }`. To change a link you must be able to read both the current and the new record.

## Events, triggers and jobs

| Event | Record | When |
| --- | --- | --- |
| `message.received` | the conversation's contact and/or deal (one event per record; no link, no record) | Message received (`reopened: true` when it reopened a resolved conversation) |
| `message.sent` | same | Reply sent from the inbox (queued) |
| `message.failed` | same | Sending failed for good (permanent error or attempts exhausted) |
| `conversation.assigned`, `conversation.status_changed`, `conversation.linked` | same | Changes to the conversation, with `changes: [{ field, old, new }]` |
| `messaging_channel.created`, `.updated`, `.deleted` | — | Channel changes |

Message payload: `conversation_id`, `message_id`, `channel_id`, `channel_kind`, `channel_name`, `direction`, `peer`, `peer_name`, `subject`, `excerpt` (up to 280 characters), `message_type`, `status`, `contact_id`, `deal_id`. Every mutation writes an audit entry in the same transaction (without the message body).

Automation triggers: **Message received** (`message.received`) and **Message sent** (`message.sent`), for people and deals.

Jobs: `messaging.send` (up to 5 attempts; permanent errors such as a closed window, a rejected token or an invalid recipient fail immediately), `messaging.inbound`, `messaging.poll` (scheduled, `0 */2 * * * *`) and `messaging.poll_channel`.

## Errors

| Code | Status | When |
| --- | --- | --- |
| `messaging_channel_not_found` | 404 | Channel does not exist (or webhook of a disabled module) |
| `messaging_channel_invalid` | 422 | Invalid channel, with `details.fields` (`config.*`, `secrets.*`, `name`, `kind`, `acting_user_id`) |
| `messaging_channel_in_use` | 409 | Deleting a channel with conversations |
| `messaging_channel_limit_reached` | 409 | More than 50 channels |
| `messaging_channel_inactive` | 409 | Replying through an inactive channel |
| `messaging_conversation_not_found` | 404 | Conversation does not exist |
| `messaging_invalid_query` | 422 | Invalid filter |
| `messaging_invalid_reply` | 422 | Empty or too long reply (WhatsApp 4096, email 100 thousand characters) |
| `messaging_invalid_assignee` | 422 | Assignee without access to the inbox |
| `messaging_record_not_accessible` | 422 | Contact/deal does not exist or is not accessible |
| `whatsapp_window_closed` | 422 | Outside WhatsApp's 24 h window (also as the message's `error_code`) |
| `whatsapp_invalid_verify_token` | 403 | Webhook verification with the wrong token |
| `whatsapp_invalid_signature` | 401 | Webhook without a valid signature |
| `messaging_payload_too_large`, `messaging_invalid_payload` | 422 | Webhook over 1 MB or without JSON |
| `messaging_acting_user_unavailable` | 503 | No active member to act for the channel (sending) |
| `rate_limited` | 429 | Webhook or test limits |

Codes stored in messages' `error_code` and in the channel test: `whatsapp_auth_failed`, `whatsapp_provider_error`, `whatsapp_delivery_failed`, `imap_auth_failed`, `imap_error`, `smtp_auth_failed`, `smtp_rejected`, `smtp_error`, `messaging_provider_timeout`, `messaging_provider_unreachable`, `messaging_tls_error`, `messaging_channel_credentials_missing`, `messaging_invalid_recipient`.

## Data

Migration `1`: `messaging_channels` (kind, name, `config` JSON without secrets, active, status, last error, last check and last read, IMAP state, acting user), `messaging_conversations` (channel, normalized `peer` — E.164 phone or lowercase email —, name, subject, contact, deal, status, assignee, unread count, last message and last received message) and `messaging_messages` (direction, provider id with `UNIQUE (channel_id, external_id)`, sender, subject, text body, type, delivery status and error, thread headers, who sent it, dates). Contact, deal and users with `ON DELETE SET NULL`.

**Export data** includes all rows of the three tables and the list of credentials of each channel (names only; the values never leave). **Remove data** deletes the three tables and the credentials of all channels.

## Limitations

- No WhatsApp template messages, no sending of media/attachments and no download of received media; email attachments are not stored.
- Email only over IMAP with implicit TLS (the channel does not offer STARTTLS on IMAP nor OAuth2/XOAUTH2), INBOX only, without importing history from before the channel, without syncing read state/folders and without storing the HTML.
- Sending is "at least once": if the process crashes between the provider's acceptance and the status write, a new attempt may resend.
- The inbox updates by periodic polling (20 s on screen), without real-time push.
- No SLAs, queues, automatic distribution rules or canned responses (outside the core; canned responses can come from other modules' `composerTools`).
- Received messages act on behalf of a user (see above) because the contract does not yet have a system scope for modules.
