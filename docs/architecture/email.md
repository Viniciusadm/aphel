# Individual email in the core

The `email` module (`back/modules/email`, enabled by default): use cases in the subfolders (`mailboxes`, `sync`, `capture`, `linking`, `reading`, `compose`, `templates`, `tracking`), SQL and providers in `store/`, routes in `http/`, DTOs in `dto/`, workers in `runtime.rs` (scheduled sending, engagement and sync), services in `service.rs` and hooks in `hooks.rs`. Module migrations: `0001_create_mailboxes.sql` (formerly `2200`–`2205` and `2212`), `0002_create_email_settings.sql` (formerly `2210`–`2211`) and `0003_create_email_templates.sql` (formerly `2230`–`2233`); `emails.read`/`emails.send` granted via `grant_to`. Web in `web/src/modules/email`.

Extension points:

- `email::Sender` service (`send`/`send_as_owner` over `&Ctx`, default `MailboxSender`, through the user's mailbox). Other modules (`quotes`, `sequences`) send through it (`email::service::sender()`), never by calling the use case directly.
- `email::OutgoingEmail` hook (filter over `OutgoingMessage`: recipients, subject, body and tracking) applied after the draft is assembled and before it is saved and sent, in a discarded transaction; with no recipient or an empty subject the send is refused (422).
- `email::InboundLinking` hook (filter over `LinkingEmail.records`) applied during sync with the records the imported message will link automatically (people and open deals); the result replaces the list.

Each user connects their own mailbox. Messages synced or sent through Aphel become threads linked to people, organizations, deals and custom objects. The shared inbox of the `messaging` module remains separate (pending: unify its email channel with the `MailboxProvider`).

## Model

| Table | What it stores |
| --- | --- |
| `mailboxes` | A user's mailbox: provider (`imap` or `<module>.<key>`, e.g. `google.gmail`), address, state (`active`, `syncing`, `error`, `disconnected`), settings without secrets (`settings`), sync cursor, initial history days (default 90) and default visibility |
| `email_threads` | A mailbox thread: subject, its own visibility (`NULL` = the mailbox's), count, snippet, date of the first and the last message |
| `email_messages` | Message: direction (`inbound`/`outbound`), kind (`message`, `bounce`, `auto_reply`), state (`received`, `draft`, `scheduled`, `sending`, `sent`, `failed`, `cancelled`), threading headers, snippet, body in a file, tracking and open, click, reply and bounce dates |
| `email_participants` | Addresses by role (`from`, `to`, `cc`, `bcc`, `reply_to`) with the resolved `contact_id` |
| `email_record_links` | Thread → record link (`RecordRef`), `automatic` or `manual` (an automatic link becomes manual when the user confirms it) |
| `email_attachments` | Message attachments, each in a `FileStorage` file |

- Idempotency: `(mailbox_id, message_id_hash)` is unique, with the SHA-256 of the normalized `Message-ID` (without `<>`). `MessageStore::insert_message` returns `false` when the message already exists.
- The body (`EmailBody { text, html }` as JSON, type `application/vnd.aphel.email-body+json`) lives in a `FileStorage` file, referenced by `body_file_id`. It never goes into the database or into events.
- Credentials (IMAP/SMTP password, OAuth tokens) live in the `SecretVault` with `SecretKey { owner_type: "mailbox", owner_id: <mailbox id>, name: "credentials" }`, as JSON (`MailboxCredentials`). They are never returned through the API, logs or audit.

## Visibility

`EmailVisibility` applies per mailbox (default) and per thread (overrides):

| Value | Mailbox owner | Other users |
| --- | --- | --- |
| `private` | everything | nothing (the thread does not even appear) |
| `metadata` | everything | subject, participants and dates, without snippet, body or attachments |
| `shared` | everything | full content |

`EmailVisibility::access_for(is_owner)` returns the `EmailAccess` (`None`, `Metadata`, `Full`). In addition, a non-owner only sees a thread if they can read at least one record linked to it (`RecordAccessChecker`), and listing by record requires read access to the record. `MessageStore::list_threads_for_record` already removes other owners' `private` threads in SQL; redacting `metadata` content is the responsibility of the reading use case.

## Providers

The `MailboxProvider` trait (`modules/email/provider.rs`) has `connect`, `test`, `sync_since(cursor)` and `send`. The `MailboxProviderRegistry` gathers the module's own IMAP/SMTP provider (`imap`) and those contributed by other modules with `app.contribute::<dyn email::MailboxProvider>(Arc::new(..))` (currently `google` and `microsoft`, which reuse the `oidc` code), read with `App::all_contributions` and key `<module>.<key>`; `MailboxProvider::extension()` returns the owning module's key. Use `available_for(scope)`/`require_for(scope, key)` to respect the enabled modules.

- Providers that refresh tokens call `account.persist_refreshed(&credentials)` right after the refresh, before using the new token: the `MailboxAccount` carries a `CredentialPersistence` (a `CredentialSink` that writes to the `SecretVault` in its own transaction, built by `service::mailboxes::credential_persistence(ctx)`), so the refreshed token is saved even if the next operation fails. A failure to save only produces a warning in the log. They still return the credentials in `SyncBatch.credentials`/`SentEmail.credentials`/`test`, and the caller saves them again (idempotent).
- Errors come as `MailProviderError` (`authentication`, `connection`, `rejected`, `unsupported`, `internal`). `is_retryable()` says whether retrying is worthwhile, and the conversion to `AppError` uses the `mail_provider_*` codes.
- OAuth providers (`MailboxConnectionKind::OAuth { start_path }`) run the flow in the owning module's routes (`/api/v1/<module>/...`) and finish by calling the mailbox service directly (`email::service::mailboxes::create`) with the user's scope, with the tokens as credentials.

## Events

`email.received`, `email.sent`, `email.opened`, `email.clicked` and `email.bounced` (`EMAIL_EVENT_TYPES`) are published **once per record linked** to the thread (`EmailEventPayload::targets()`), or once without a record when there is no link. This way the timeline, automations and scoring receive each record. The payload (`EmailEventPayload::to_value`) carries ids, direction, kind, visibility, `is_reply` and the linked records, **without** subject, addresses or body. Anyone who needs the content reads it through the use case, respecting visibility.

Mailboxes publish `mailbox.connected|updated|disconnected` and manual links publish `email_thread.linked|unlinked`, all with audit.

## Permissions

`emails.read` (view the records' threads) and `emails.send` (send from the record), granted to the system roles. The mailbox and personal preferences belong to the user. Organization settings (tracking, capture, blocklist, organization templates) require `settings.manage`.

## Scheduling without a user scope

`MailboxStore::due_for_sync` is the only query without an `AccessScope`, like `ApiTokenRepository::find_by_hash`: it returns all pending mailboxes for the sync job, which builds the owner's scope before any other read.

## Sync and relationship capture (track A)

### Mailboxes

`/api/v1/mailboxes*` (session or API token; each user only sees and changes their own mailboxes). The use cases are functions over `Ctx` in `modules/email/service/{mailboxes,sync,capture,reading,linking}.rs`, with SQL in the concrete stores of `modules/email/store` (`MailboxStore`, `MessageStore`, `LinkStore`, `SettingsStore`, `DirectoryStore`); the sync jobs and the engagement consumer live in `modules/email/sync/jobs.rs`.

- Creating calls `MailboxProvider::connect`, which validates and returns the normalized address; the returned credentials go to the `SecretVault` (`mailbox`/`<id>`/`credentials`) and the saved `settings` goes through `public_settings` (keys containing `password`, `secret`, `token` or `credential` are never stored or returned). An address already connected by the same user → 409 `mailbox_already_connected`.
- `PATCH` with `settings` or `credentials` reconnects (the address cannot change: 422 `mailbox_address_mismatch`) and resumes a paused sync.
- `mailbox.connected|updated|disconnected` with audit; in the diff the credentials only appear as `"[redacted]"`.
- Deleting removes the mailbox (cascading to threads, messages, participants, links and attachments), the credentials and the body and attachment files (rows in `files` and objects in `FileStorage`).

### Core IMAP/SMTP provider (`imap`)

`modules/email/store/imap`. `settings`: `imap_host`, `imap_port`, `imap_security` (`tls`, `starttls` or `none`), `smtp_host`, `smtp_port`, `smtp_security`, `username` and, optionally, `address` (when the username is not an email), `smtp_username` and `sent_folder`. `credentials`: `password` and, optionally, `smtp_password`.

- `connect`/`test` log in to IMAP (EXAMINE on INBOX) and test SMTP; the sent folder is discovered through the `\Sent` attribute or common names (`Sent`, `Sent Items`, `Enviados`...).
- `sync_since` reads INBOX and the sent folder with `EXAMINE` + `BODY.PEEK` (marks nothing as read). The cursor stores `UIDVALIDITY` and the last UID per folder; without a cursor (or with a different `UIDVALIDITY`) it searches `SINCE` the initial history date (the mailbox's `history_days`). Messages above `EMAIL_SYNC_MAX_MESSAGE_MB` come in with headers only. The sent folder sets `sent_by_account`. Parsing (`mail-parser`) fills `content_type` (with `report-type`), `Auto-Submitted` and `report_for_message_id` (from the `message/rfc822` or `text/rfc822-headers` part of a bounce).
- `send` builds the message with `lettre` (text + HTML in `multipart/alternative`, attachments and inline attachments, `Message-ID`, `In-Reply-To`, `References` and extra headers, ignoring the protected ones). After the SMTP send, the same message is saved via `APPEND` (with `\Seen`) to the sent folder (the mailbox's `sent_folder` or one discovered through the `\Sent` attribute/common names), except when the SMTP server already keeps the copy by itself (`smtp.gmail.com`, `smtp.googlemail.com`, `smtp.office365.com`, `smtp-mail.outlook.com`). An `APPEND` failure only produces a warning in the log: the send has already happened. The next sync finds the copy and ignores it by `Message-ID`.
- Hosts that resolve to loopback or a private network are refused (`mail_provider_rejected`) unless `EMAIL_ALLOW_PRIVATE_NETWORKS=true`.

### `email.sync` job

- `email.sync` (scheduled every minute) uses `MailboxSchedule::due_for_sync(now - EMAIL_SYNC_INTERVAL_MINUTES)`, discards paused mailboxes (`mailbox_sync_failures`) and enqueues one `email.sync.mailbox` per mailbox with `unique_key = email.sync.mailbox:<id>` (at most one per mailbox in the queue). `POST /mailboxes/{id}/sync` enqueues the same job immediately.
- `email.sync.mailbox` builds the owner's scope (`load_access` + `build_access_scope`; a deactivated owner ends the job), sets `syncing`, runs up to `EMAIL_SYNC_MAX_BATCHES` batches of `EMAIL_SYNC_BATCH_SIZE`, saves the cursor after each batch and finishes in `active` (with `last_synced_at` only when nothing is left, so the next run continues the history).
- Provider error: state `error` with the reason (no secrets). Authentication pauses the mailbox until the user fixes it (PATCH with credentials, a successful `test` or a manual `sync`); connection errors wait 5 min × 2^failures (maximum 6 h).
- Idempotency: `message_exists` before any work and `insert_message` (unique by `(mailbox_id, Message-ID hash)`). Without a `Message-ID`, the synthetic id is `aphel-<sha256 of date|sender|subject|provider id>@sync.invalid`.
- Threading by `In-Reply-To`/`References` (`find_thread_by_message_ids`), otherwise a new thread with the mailbox's default visibility. Body (`EmailBody`) and attachments go to `FileStorage` + `files` (owner = mailbox owner). After saving: links, `set_participant_contact`, `refresh_thread` and the events, all in the same transaction.
- Events: `email.received` (inbound) or `email.sent` (message found in the mailbox and sent outside Aphel), `Actor::System`, `EmailEventPayload` with `extra.source = "sync"`, one publication per linked record. Each imported message also writes a system audit entry (`Actor::System`, action = event type, record `email_message`) in the same transaction, with `mailbox_id`, `thread_id`, `direction`, `kind` and `source = sync`, never subject, addresses or body.

### Blocking and internal participants

A message is only imported if it has at least one external, non-blocked participant. Internal: the organization's mailbox addresses, active members' emails and their domains, except personal domains (`PERSONAL_DOMAINS` in `modules/email/capture`: gmail.com, outlook.com, hotmail.com, yahoo.com(.br), icloud.com, uol.com.br, bol.com.br, terra.com.br...). Blocked: `email_blocklist` (`/api/v1/email-blocklist`, `settings.manage`), by address or domain (a domain covers subdomains). Events `email_blocklist.added|removed` with audit.

### Links

- Automatic, for each imported message: external participant → contact by `email` (deleted or archived people are excluded); contact → primary company and additional organizations (`people_organizations`) that are not deleted/archived → the contact's `open` deals (`deals.contact_id` or `deal_contacts`) when `link_open_deals` is enabled.
- Manual: `POST /email-threads/{id}/links` and `DELETE .../links/{record_type}/{record_id}` require full access to the thread and read access to the record; they publish `email_thread.linked|unlinked` (record = the linked record, payload with `thread_id`) with audit. A confirmed automatic link becomes manual. A new message can recreate a removed automatic link.

### Capture (disabled by default)

`/api/v1/email-settings/capture` (`settings.manage`, `email_capture_settings.updated` with audit): `create_contacts`, `create_companies`, `link_open_deals` (enabled by default) and `initial_history_days` (default `history_days` for new mailboxes). With `create_contacts`, an unknown sender (inbound) or `to`/`cc` recipient (outbound) becomes a contact when there is already a message in the opposite direction with the same mailbox (two-way exchange). With `create_companies`, the company is found by domain or created (name derived from the domain); personal domains never become organizations. Everything goes through the people and organizations use cases with the mailbox owner's scope (regular `contact.created`/`company.created` events, `source = email`); previous threads with the address are linked to the new contact. The created contact only gets `lifecycle_stage = lead` when the `lead` option exists and is active in the people' `lifecycle_stage` field; otherwise it is created without a stage. A permission failure for the owner only produces a warning in the log.

### Computed fields

Contacts and organizations have `last_contacted_at`, `last_email_received_at`, `last_email_sent_at`, `interaction_count` and `last_contacted_by` (the user who owns the mailbox of the most recent message), table columns (migrations `2213`/`2214`) declared as system fields (filterable and sortable through `RecordQuery`, rejected by `CustomFieldWriter` and excluded from import). The `email.engagement` consumer recalculates the record's values from `email_record_links` + `email_messages` (only `received`/`sent` of kind `message`) on each `email.received|sent` and `email_thread.linked|unlinked`; the recalculation is idempotent and does not change `updated_at`. Decision: the values also aggregate `private` and `metadata` threads, because they are relationship metadata (when contact happened, how many messages and who spoke last) and do not depend on who is reading. The recalculation SQL reads only `direction`, `sent_at`/`created_at`, the message id and the mailbox owner; never subject, participants, snippet, body or attachments, so nothing from the content of a private thread appears on the record. The `/people` and `/organizations` responses do not include these fields yet (pending).

### Reading and timeline

- `GET /email-threads?record_type=&record_id=` (read access to the record), `/email-threads/mine`, `/email-threads/{id}` and the attachment download require `emails.read`. A non-owner only sees non-`private` threads with at least one readable linked record; with `metadata`, snippet, body and attachments come out empty; `tracking` and `failure_reason` are owner-only; HTML is sanitized with `ammonia`. Only the owner can change visibility (`email_thread.visibility_changed` with audit).
- Email files (body and attachments not linked to records) can only be downloaded by the owner at `/api/v1/files/{id}` (treated as a generated file in `generated_kind`).
- Timeline: `email.*` and `email_thread.*` events only appear on the timeline of the event's own record (without repeating those of people and deals on the company), are hidden for users without `emails.read` and for other owners' `private` threads, and the item gets `metadata.email = { thread_id, subject, direction, access, snippet (null in metadata), from }`, read at query time (`modules/email/store/timeline.rs`).

### `messaging` email channel

The shared inbox email channel (`modules/messaging/src/channels/email.rs`) has no IMAP/SMTP stack of its own: it builds a `MailboxAccount` from the channel's settings and passwords and uses the registry's `imap` provider (`email::store::mail_providers()`) for `test`, `send` (with the `APPEND` to sent) and reading INBOX (`sync_since` with an INBOX-only cursor). The channel's error codes (`imap_auth_failed`, `smtp_rejected`...) are derived from `MailProviderError`. The provider's network policy (`EMAIL_ALLOW_PRIVATE_NETWORKS`) also applies to the channel.

### Local testing

Mailpit has no IMAP. To validate sync: `docker run -d -p 127.0.0.1:3025:3025 -p 127.0.0.1:3143:3143 -e GREENMAIL_OPTS='-Dgreenmail.setup.test.smtp -Dgreenmail.setup.test.imap -Dgreenmail.hostname=0.0.0.0 -Dgreenmail.users=ana:secret123@acme.test -Dgreenmail.users.login=email' greenmail/standalone:2.1.2`, a server with `EMAIL_ALLOW_PRIVATE_NETWORKS=true` and a mailbox with `imap_security`/`smtp_security` `none`, ports 3143/3025.

### Pending

- Expose the computed fields in the people and organizations responses.

Gmail and Outlook (providers `google.gmail` and `microsoft.outlook`): see the email sections of [`docs/modules/google.md`](../modules/google.md) and [`docs/modules/microsoft.md`](../modules/microsoft.md).

## Sending, templates and tracking (track B)

Code in `back/modules/email/{compose,templates,tracking}`, MySQL adapters in `back/modules/email/src/store/sending`, routes in `back/modules/email/src/http/{emails,email_templates,email_tracking,email_tracking_public}.rs`, module migration `0003_create_email_templates.sql` (formerly `2230`–`2233`). The use cases are functions over `Ctx` in `modules/email/service/{compose,templates,tracking}.rs`, with the stores `OutboundStore`, `TemplateStore` and `TrackingStore` (`store/sending`); reading records for the variables lives in `modules/email/service/records.rs` (`load_record`/`readable_fields`), using the modules' use cases (`HandlerAutomationGateway::load_record`/`fields`, which read through the type's handler), so the read scope, the `CustomFieldReader` and `FieldSet::for_scope` apply (a hidden field does not become a variable).

### Sending (`POST /api/v1/emails`, `emails.send`)

1. Mailbox: `mailbox_id` (always the user's own), the one of the thread being replied to or the user's first `active` mailbox (`no_mailbox_available` without a mailbox; `mailbox_unavailable` with a `disconnected` mailbox).
2. `record` (contact, company, deal or `custom.<key>`) requires read access to the record. In a reply without `record`, the first readable record linked to the thread (contact first) provides the variables.
3. Reply (`reply_to_message_id`): the message must belong to one of the user's mailboxes; it keeps the thread, `In-Reply-To` = the replied `Message-ID`, `References` = its references + that id, subject `reply_subject` (`Re: ...`).
4. Subject and body come from the request or from the template (`template_id`), always rendered on the server (variables escaped in HTML, HTML sanitized; without `body_text`, the text is derived from the HTML).
5. Attachments (`attachment_file_ids`): only files the user can download (`FileUseCases::download`: uploaded by them, linked to a readable record or `settings.manage`); up to 20 files and 25 MB. The attachment references the same `file_id` (no copy).
6. `Message-ID` = `<uuid v7 without hyphens>@<sender domain>`. Convention: the database and `OutgoingEmail` store ids **without** `<>`; the provider adds the brackets (the core IMAP/SMTP uses `bracket`).
7. One transaction saves a new thread (if it is not a reply), the body file (`EmailBody` in `FileStorage`, a row in `files` with the sender as `uploaded_by`), the `sending` (or `scheduled`) message with participants (`contact_id` of people by email), the thread's `automatic` link to the source record and to the recipient people, and `refresh_thread`.
8. After the commit, the provider (`state.mail_providers.require_for(scope, mailbox.provider)`, credentials from the vault) sends. Success: `sent`, `sent_at`, `provider_message_id`, refreshed credentials (`SentEmail.credentials`) saved to the vault, `email.sent` once per linked record + audit (`email_message`: status, subject, recipients, number of blind copies and attachments, tracking; never the body). Failure: `failed` + `failure_reason`, `email.failed` and audit, and the API responds with the `mail_provider_*` error.

### Scheduled sending

`scheduled_at` more than 5 s in the future (up to 365 days) saves `scheduled`, publishes `email.scheduled` (without a record, so it does not appear on the timeline) + audit and enqueues `email.send_scheduled` with `run_at`, `unique_key = email.send_scheduled:<id>` and `requested_by`. The job (`ScheduledEmailHandler`) reloads the author's current scope (`build_access_scope`); without `emails.send` or with a deactivated author, the message becomes `failed`. The `scheduled → sending` transition is conditional (`UPDATE ... WHERE status = 'scheduled'`), so the job is idempotent and a concurrent cancellation wins. A transient provider error (`is_retryable`) returns the message to `scheduled` and the job retries up to `max_attempts`; a permanent error or the last attempt marks it `failed`. The scheduled `email.sent` goes out with `Actor::System` and `requested_by` in the payload. `POST /api/v1/emails/{id}/cancel` (mailbox owner or author) changes `scheduled → cancelled` (`email_not_cancellable` if it has already gone out) and publishes `email.cancelled`.

### Templates (`email_templates`)

- Scope `organization` (writing requires `settings.manage`) or `personal` (only the owner sees and edits). A stable `key` (snake_case, unique among the organization's or the owner's templates, generated from the name when absent), an optional `record_type` (filters the list), subject and sanitized HTML. Events `email_template.created|updated|deleted` with audit.
- Variables: `{{contact.<field>}}`, `{{company.<field>}}`, `{{deal.<field>}}`, `{{custom.<key>.<field>}}`, `{{record.<field>}}` (the source record), `{{user.display_name}}`, `{{user.email}}` (the sender). A contact brings its company; a deal brings contact and company (each only if readable). Custom fields are included by key. A missing variable becomes empty text and appears in the preview's `missing_variables`.
- `GET /api/v1/email-templates/variables?record_type=` lists the readable variables (`FieldSet::for_scope`).
- Configuration packages: `email_templates` section (`EmailTemplatesSection`), organization templates only, reconciled by `key`; installing creates or updates and never deletes.

### Open and click tracking

- Organization preferences in `email_tracking_settings` (`GET`/`PUT /api/v1/email-settings/tracking`): `opens_enabled`, `clicks_enabled`, `default_on`, all disabled by default. Per send, `tracking` (default `default_on`) only applies with HTML and with some tracking enabled.
- **Objection** (LGPD, art. 18, § 2): the privacy module did not have the concept, so the minimal check lives here: `email_tracking_objections` stores only the SHA-256 of the normalized address and a masked label (`mask_email`), managed with `privacy.manage` at `/api/v1/email-settings/tracking/objections` (address in the body, never in the URL; events `email_tracking.objection_added|removed` with audit). If any recipient (to, cc or bcc) has an objection, the whole message goes out without tracking; opens and clicks of older messages to that address are no longer recorded.
- Token: `base64url(message id ‖ link position or 0xFFFF for an open ‖ HMAC-SHA256 truncated to 16 bytes)`. The HMAC key is random, per installation, in the vault (`SecretKey::organization("email_tracking_key")`, `owner_id = "default"`), created when tracking is enabled or on the first tracked send. The URL contains no address, name or subject.
- Public routes (no session, `GET`, outside CSRF and idempotency): `GET /api/v1/e/o/{token}` always responds with a 1×1 GIF (`no-store`); `GET /api/v1/e/c/{token}` redirects (302) to the link stored in `email_tracked_links` at send time. Only `http(s)` URLs that were in the sent body are rewritten and reachable, so there is no open redirect; an invalid token responds 404 `email_tracking_link_not_found`. The base address is the public address saved in Settings → General.
- Recording: `record_engagement` increments `open_count`/`click_count` on each open/click and fills `opened_at`/`clicked_at` on the first one. **`email.opened` and `email.clicked` are only published the first time** for each (with audit); the counts stay on the message. Ignored when the request carries the session of the mailbox owner or the author (opening in Aphel itself; the stored body never has a pixel or rewritten links, which only exist in the sent copy), when the organization has disabled that tracking or when there is an objection.

### Reply and bounce

The `email.reply_detection` consumer (`EmailEngagementConsumer`, stable name) listens to `email.received`. For an `inbound` message, it looks for (by `report_for_message_id` when it is a `bounce`, then `In-Reply-To` and `References`, from newest to oldest) an `outbound` `sent` message **from the same mailbox**. A reply (`kind = message`) sets `replied_at` (no event of its own; audit `email.replied`); a `bounce` sets `bounced_at` and publishes `email.bounced` once per linked record (`Actor::System`, `bounce_email_id` in the payload). Auto-replies (`auto_reply`) are ignored. It is idempotent: only the first mark publishes.

### Automation

- `send_owner_email` action ("Send email from the owner's mailbox", `retry_safe = false`, requires `emails.send` and read access to the type): sends from the first active mailbox of the **record owner** (`owner_id`), to the record's email field (default `email`) or fixed addresses (`to: [{ "type": "field", "field": "email" } | { "type": "address", "address": "..." }]`), with `template_id` (template variables) or `subject` + `body` (`{{record.*}}` templates of the automation engine), optional `tracking`. It goes through the `AutomationGateway::send_owner_email` port, implemented in `ApiAutomationOperations` (`kernel/api/automation_operations.rs`) with `email::service::compose::send_as_owner` (the same use case as the API; actor `Actor::Automation` and `causation` in the events). Without a recipient, the step ends with `sent: false`.
- Triggers `email.received`, `email.sent`, `email.opened`, `email.clicked`, `email.bounced` for people, organizations, deals and custom objects: they fire for the record of each publication of the event (one per linked record).

### Local testing of track B

Send through an IMAP/SMTP mailbox whose SMTP is a Mailpit with authentication accepted (`MP_SMTP_AUTH_ACCEPT_ANY=1`, `MP_SMTP_AUTH_ALLOW_INSECURE=1`) and check `Message-ID`, `In-Reply-To`, `References`, subject and tracked HTML through the Mailpit API (`/api/v1/message/{id}/headers`). For reply/bounce, deliver to the test IMAP a message with `In-Reply-To` (or from `MAILER-DAEMON` with `References`) pointing to the sent `Message-ID` and sync the mailbox.

### Track B pending items

- Sent attachments reuse the `file_id` uploaded by the user; since `generated_kind` treats `email_attachments` files without a record attachment as email content, only the user who uploaded the file can keep downloading it through `/api/v1/files/{id}`.
- The committed `openapi.json` has not been regenerated (new routes from several tracks); regenerate during integration.
