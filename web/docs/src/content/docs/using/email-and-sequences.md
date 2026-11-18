---
title: Email and sequences
description: Connect your mailbox, send and track emails from records, use templates, and run automated follow-up sequences.
sidebar:
  order: 8
---

Aphel can sync the emails you exchange with customers onto their records, let you send emails from a record, and run sequences: scheduled series of emails and tasks that follow a contact until they respond.

## Availability

| Area | Module | Recommended in |
| --- | --- | --- |
| Mailboxes, sending, templates, tracking | `email` | Agency, Consultancy, Freelancer, Other business, Real Estate, B2B Sales |
| Sequences | `sequences` | Agency, B2B Sales |
| Gmail and Outlook mailboxes | `google`, `microsoft` (with `email`) | Most blueprints |
| Shared inbox for WhatsApp and email | `messaging` | Real Estate |

These modules are optional. An administrator can turn them on from [Modules](/_aphel/docs/admin/modules/); see also [Blueprints](/_aphel/docs/admin/blueprints/).

## Permissions

| Permission | What it allows |
| --- | --- |
| `emails.read` | See email conversations on records and the Emails tab. |
| `emails.send` | Send emails, use templates (including your personal ones), see tracking settings. |
| `settings.manage` | Organization templates, the blocklist, and tracking and capture preferences. |
| `privacy.manage` | Manage addresses that objected to tracking. |
| `sequences.read` | See sequences and their enrollments. Granted by default to admin, manager and member. |
| `sequences.enroll` | Enroll contacts, pause, resume and remove enrollments. Granted by default to admin, manager and member. |
| `sequences.manage` | Create, edit and delete sequences. Granted by default to admin only. |
| `messaging.view`, `messaging.reply`, `messaging.manage_channels` | See, reply to and manage channels of the shared inbox. |

To read or send on a record you must also be able to read that record.

## Connect your mailbox

Every user connects their own mailbox. Open **Settings**, account section, **Email and communication** group, **Email mailboxes**.

1. Under **Connect a mailbox**, choose **Connect Google** or **Connect Microsoft** (shown when those modules are configured), or choose the IMAP/SMTP option.
2. For Google or Microsoft, accept the permissions at the provider.
3. For IMAP/SMTP, enter the display name, the incoming (IMAP) server, port and security (SSL/TLS, STARTTLS or none), the outgoing (SMTP) server, port and security, and your username and password. The password is stored encrypted and never shown again.
4. Choose **Initial history**: how many days back to import on the first sync (1 to 3650, 90 by default).
5. Choose the **default conversation visibility** (see below) and select **Connect**.

The first sync starts immediately, and then runs about every 5 minutes. From the same screen you can **Test connection**, **Sync now**, edit the mailbox, or remove it. Removing a mailbox deletes its synced conversations and stored password from Aphel; nothing is deleted at your provider.

A mailbox has one of these statuses: Active, Syncing, Error or Disconnected. You cannot send from a disconnected mailbox; reconnect it first.

### Who can read your emails

Each mailbox has a default visibility, and you can override it per conversation (open the conversation and change **Visibility of this conversation**; only the mailbox owner can).

| Visibility | What teammates see |
| --- | --- |
| Private | Nothing, not even in the timeline. |
| Metadata only (default) | Subject, participants and dates, but not the content, preview or attachments. |
| Shared | The full conversation, to anyone who can see the linked record. |

You always see your own conversations in full.

## Emails on records

Conversations are linked to people, organizations and deals (and custom objects) automatically from their participants. Open the record and use its **Emails** tab to read them.

- **Link record** attaches a conversation to another record by hand; **Unlink** removes the link (the message stays in the mailbox).
- Remote images in received emails are blocked until you choose **Show images**.
- **Reply** answers from the mailbox that received the message.

## Send an email

1. Open a record and select **New email** on its Emails tab.
2. Choose **From** (your connected mailbox), then fill **To**, optionally **Cc** and **Bcc**, and the **Subject**.
3. Optionally pick a **Template**. Choose the body format: **Formatted**, **Text** or **HTML**. Use **Preview** to check variables.
4. Attach files if needed.
5. Choose **Send**, or use **Schedule sending** and pick a date and time, then **Schedule**.

| Limit | Value |
| --- | --- |
| Recipients (To, Cc and Bcc together) | 50 |
| Subject | 998 characters |
| Body | 2 MB |
| Attachments | 20 files, 25 MB in total |
| Scheduling | Up to 365 days ahead |

A message can be Draft, Scheduled, Sending, Sent, Failed or Cancelled. A scheduled email can be cancelled with **Cancel sending** until it starts being sent. Sent emails appear on the record's timeline.

## Templates

Templates keep the usual messages ready to reuse. Variables written as `{{...}}` are filled with data from the record when you compose.

- **Personal templates**: in **Email mailboxes**, tab **Personal templates**. Only you see them. Requires `emails.send`.
- **Organization templates**: in **Settings**, organization section, **Team email**, tab **Templates**. Everyone who can send emails can use them. Creating them requires `settings.manage`.

A template has a name, a key (lowercase letters, numbers and underscore; used by configuration packages), an optional record type it is meant for, a subject and an HTML body. Use **Insert variable** to pick variables. Deleting a template does not change emails already sent. Body is limited to 512 KB.

## Tracking opens and clicks

Administrators set tracking in **Settings**, organization section, **Team email**, tab **Tracking and capture**:

| Setting | Effect |
| --- | --- |
| Track opens | Adds an invisible image to the message. Some email clients block images, so an open may not be counted. |
| Track clicks | Links pass through an Aphel redirect. |
| On by default for new emails | The composer's **Track opens and clicks** option starts on. Senders can still turn it off per email. |

Tracking only applies to emails with an HTML body, and only if opens or clicks are enabled. Recipients who objected to tracking are never tracked. Only the sender sees the tracking results (opened N times, clicks, replied, bounced); they also appear as events on the timeline.

## Blocklist and contact capture

Administrators (`settings.manage`) tune what is synced:

- **Blocklist** tab: domains and addresses whose messages are not synced or linked to records. Good for personal domains and newsletters. Removing an entry syncs future messages again; older ones are not recovered.
- **Tracking and capture** tab, **Contact capture**. All options start off:
  - Create people for unknown senders (only when messages were exchanged both ways).
  - Create organizations from the email domain (personal and blocklisted domains are ignored).
  - Link conversations to the contact's open deals.
  - A default initial history, suggested when mailboxes are connected.

:::note
**System email** in settings is a different thing: it is the system's own sender for notifications and scheduled report emails. It is not your mailbox.
:::

## Sequences

A sequence is a series of steps that Aphel runs for each enrolled contact. Open **Sequences** in the sidebar.

### Step types

| Step | What happens |
| --- | --- |
| Automatic e-mail | Sent from the mailbox of whoever enrolled the contact, inside the sending window. Later emails reply in the same conversation. |
| Manual e-mail | Creates an email task for the enroller, with a draft ready to review and send. |
| Call | Creates a call task for the enroller. |
| LinkedIn | Creates a task to reach the contact on LinkedIn. |
| Task | Creates a generic task for the enroller. |
| Wait | Waits a number of business days (1 to 365, in the organization time zone) before the next step. |

A sequence can have up to 30 steps and needs at least one step that is not a wait.

### Create a sequence

Requires `sequences.manage`.

1. Select **New sequence**.
2. Enter a name (up to 200 characters) and optional description. Leave **Active sequence** on.
3. Set the sending window (start 08:00 and end 18:00 by default) and **Maximum e-mails per mailbox per day** (50 by default, up to 2000).
4. Add steps with **E-mail**, **Wait** and **Task**, and order them. For emails, pick a template or write the subject and body; text you fill in takes precedence over the template.
5. Select **Save**.

Automatic emails go out only on business days, inside the window, in the organization time zone. A contact never gets two sequence emails on the same day. When a limit is reached, the email moves to the next valid time. Deactivating a sequence holds all its enrollments until it is active again.

### Enroll contacts

Requires `sequences.enroll`. You can enroll:

- One contact: open the contact, **Sequences** tab, **Enroll in sequence**. Choose an active sequence and the mailbox to send from (default: your first connected mailbox).
- Many contacts: the **Enroll in sequence** bulk action on a list.
- Automatically: an automation with the enroll action (see [Automations](/_aphel/docs/using/automations/)). Emails go from the record owner's mailbox by default.

Enrolling fails if the sequence is inactive or empty, the contact is already in it, the contact has no email (when the sequence sends emails), the contact objected to tracking, or no mailbox is available.

### Follow enrollments

Open a sequence to see its steps, its **Metrics per step** (sent, opened, replied, bounced, exits) and its **Enrollments**. An enrollment is In progress, Paused, Completed or Exited. Use **Pause**, **Resume** or **Remove from sequence** on a row.

A contact leaves the sequence automatically when they reply, book a meeting, a related deal is won or lost, an email bounces, they object to tracking, or the contact is deleted. Other exits: removed manually, the contact has no email or is unavailable, the enroller left the organization, or sending failed. The reason is shown on the enrollment. To start over, enroll the contact again.

Deleting a sequence removes its enrollments and history; emails and tasks already created stay.

## Shared inbox (messaging)

If the `messaging` module is active, **Inbox** (under **My work**, at the messaging address) shows team conversations from WhatsApp (Meta Cloud API) and email channels. This is different from the task **Inbox** described in [Tasks and calendar](/_aphel/docs/using/tasks-and-calendar/#the-task-list).

- Filter by status (Open, Pending, Resolved), assignee (All, Mine, Unassigned), channel and unread.
- Assign a conversation, change its status, and link it to a contact and a deal so new messages show on their timeline.
- Reply from the panel. WhatsApp only accepts free text within 24 hours of the customer's last message; after that you must use an approved template.
- **Channels** (needs `messaging.manage_channels`) connects WhatsApp numbers and mailboxes. An organization can have at most 50 channels, and a channel with conversations can only be deactivated, not deleted.

## Tips

- The first sync can take a while for mailboxes with a lot of history. Lower the initial history if you only need recent conversations.
- Not seeing a teammate's email on a record? Its visibility is probably Private or Metadata only.
- A sequence that sends emails needs a connected mailbox for whoever enrolls the contact.
