---
title: Tasks and calendar
description: Plan follow-ups with tasks, log calls and meetings, write notes, and see everything on the inbox, calendar and timeline.
sidebar:
  order: 7
---

Tasks are how Aphel tracks what has to happen next, and what already happened, with a person, an organization or a deal. This page covers tasks, activity types, the Inbox and Calendar screens, notes, reminders, the record timeline, calendar sync with Google or Microsoft, and the Calls area.

## Which modules provide what

| Area | Module | Notes |
| --- | --- | --- |
| Tasks, activity types, Inbox | `tasks` | Recommended in every blueprint, because all blueprints extend the CRM blueprint. |
| Calendar | `calendar` | Recommended in every blueprint. It shows entries contributed by other modules; today those are tasks. |
| Notes | `notes` | Required by the CRM blueprint. |
| Timeline | `timeline` | Recommended in every blueprint. |
| Task reminders and "due today" alerts | `reminders` | Recommended in every blueprint. Without it, tasks still work but no reminder notification is sent. |
| Calendar sync | `google`, `microsoft` | Recommended in most blueprints (see [Calendar sync](#calendar-sync-with-google-and-microsoft)). |
| Calls | `telephony` | Recommended only in the Real Estate and B2B Sales blueprints. |

Modules that are recommended but not yet active can be turned on by an administrator. See [Modules](/docs/admin/modules/) and [Blueprints](/docs/admin/blueprints/).

## Permissions

| Permission | What it allows |
| --- | --- |
| `tasks.read` | See tasks, the Tasks and Inbox screens, the Tasks tab on records, and activity types. |
| `tasks.create` | Create tasks, log activities, create from the Calendar, check in, log calls. |
| `tasks.update` | Edit, complete, reopen, postpone, reassign and reschedule tasks. |
| `tasks.delete` | Delete a task. |
| `tasks.export` | Export tasks. |
| `calendar.read` | Open the Calendar screen. Granted by default to the admin, manager and member roles. |
| `notes.read`, `notes.create`, `notes.update`, `notes.delete` | See, add, edit and delete notes. |
| `settings.manage` | Create, edit, reorder, activate and deactivate activity types. |

Each of the `tasks.*` and `notes.*` permissions is granted with a scope (own, team or organization) set on the role, so two people with `tasks.read` can still see different tasks. See [Roles and permissions](/docs/admin/roles-and-permissions/).

You also need to be able to read the linked record. A task linked to a deal is only reachable by someone who can read that deal.

## Tasks

A task is an activity with a type, an owner and, usually, a due date. The same record is used for two things:

- **Task**: something planned for later. It starts as Open.
- **Logged activity**: something that already happened, such as a call. It is saved as Done immediately.

### Fields

| Field | Details |
| --- | --- |
| Title | Required, up to 255 characters. |
| Type | Required. One of the active activity types. |
| Priority | Low, Normal, High or Urgent. |
| Due | Date and time. Needed for reminders, repetition and the Calendar. |
| When it happened | Logged activities only. Leave empty to log it now. |
| Owner | Defaults to you. Must be an active member of the organization. |
| Duration (minutes) | Whole number from 0 to 10080. |
| Description | Up to 20,000 characters. |
| Reminder | See [Reminders](#reminders). |
| Repeat | See [Repeating tasks](#repeating-tasks). |

A task has one status: **Open**, **Done** or **Canceled**. A task can be linked to a record (up to 20 linked records).

### Create a task

1. Open **Tasks** under **My work** and select **New task**. To attach the task to a record, open the person, organization or deal and use **New task** from that record instead.
2. Fill in the title, type and, if needed, priority, due date, owner and reminder.
3. Select **Create task**.

### Log something that already happened

1. Open **Tasks** and select **Log activity**, or use **Log activity** on the record's Tasks tab.
2. Choose the type (for example Call or Meeting), enter a title, and set **When it happened** if it was not just now.
3. Select **Log**. A logged activity is created as Done.

### My tasks and All tasks

**Tasks** has two sections.

- **My tasks** lists your open tasks grouped as Overdue, Today, Upcoming and No due date. Tick the checkbox on a row to complete it, and untick it to reopen. The list loads in pages; use **Load more**.
- **All tasks** is a full record list with the usual filters, columns and saved views (see [Records and views](/docs/using/records-and-views/)). Select several rows and use **Complete N selected** to finish them together.

Open a task to see its detail page. From there you can **Complete** or **Reopen** it, change its **Schedule**, **Delete** it (permanent), and use the **Notes** and **Files** tabs.

### Completing and reopening

Completing a task sets its completion time to now. Reopening clears it. Completing a task that is already Done, or reopening one that is already Open, fails with a message that the activity was already in that state.

### Repeating tasks

A task with a due date can repeat. Choose **Repeat** and set:

| Option | Details |
| --- | --- |
| Frequency | Daily, Weekly, Monthly or Yearly. |
| Every | An interval from 1 to 365 (every N days, weeks, months or years). |
| Weekdays | Weekly only. With none selected, it repeats on the weekday of the due date. |
| Day of the month | Monthly only. Shorter months use their last day. Blank uses the due date's day. |
| Ends | Never, on a date, or after a number of times (1 to 1000). |

The schedule follows the organization time zone. Only one occurrence is open at a time: **completing an occurrence creates the next one**. If you reopen the completed occurrence, the next one is discarded when it is still untouched. If the owner is no longer a member when the next occurrence is created, it is created without an owner.

To change a repeating task, open it and select **Schedule**. Choose **This occurrence** or **This and following**, or select **End recurrence**.

### Reminders

When a task has a due date, you can pick a reminder: None, At the due time, 5, 15 or 30 minutes before, 1 hour before, or 1 day before. New tasks default to 1 hour before.

The task owner receives a notification at that time. In addition, owners get a "due today" notification each morning at 8:00 (organization time zone) for their open tasks due that day. Both are sent by the `reminders` module. See [Personal settings](/docs/using/personal-settings/) for notification preferences.

Assigning a task to someone also notifies them that it was assigned.

:::note
The `reminders` module also stores standalone reminders (a date plus an optional note and record), and notifies you when they are due. The web app does not currently have a screen to create them; they are available through the API.
:::

## Activity types

Activity types classify tasks and appear on the Calendar and the timeline. A new workspace from the CRM blueprint includes the types Call, Meeting, Email, Message, Visit and Task.

Administrators manage them in **Settings**, organization section, **Fields and records** group, **Activity types** (requires `settings.manage`).

- **New type**: choose a name (up to 140 characters), an icon and a color. The key is generated from the name and never changes afterward.
- Drag types to reorder them.
- Deactivate a type instead of deleting it. Inactive types cannot be chosen for new tasks. At least one type must stay active.
- An organization can have at most 100 types.

## Inbox

**Inbox** (under **My work**) shows open tasks in four groups, most urgent first: Overdue, Today, Upcoming and No due date. It needs `tasks.read`.

Filters at the top:

| Filter | Options |
| --- | --- |
| Activities of | Mine, Team, All |
| Team | With Team selected, pick a team or "My teams" |
| Sort by | Due date, Priority, Deal value |

Each row has quick actions: **Complete**, **Postpone 1 day**, **Postpone 1 week**, and **Reassign**. Every quick action shows an **Undo** option for a moment.

:::note
Inbox shows up to 500 open tasks. If there are more, a message says "Showing N of M" and you should filter or complete tasks to see the rest. "All" only includes tasks you are allowed to see. Deal value is hidden if your role cannot see deal amounts.
:::

:::caution
If the `messaging` module is also active, it adds its own menu entry named **Inbox** for shared WhatsApp and email conversations. That is a different screen. See [Email and sequences](/docs/using/email-and-sequences/#shared-inbox-messaging).
:::

## Calendar

**Calendar** (under **My work**) shows tasks that have a due date, by **Month**, **Week** or **Day**. It needs `calendar.read` plus `tasks.read` to see any tasks.

- Use **Today**, **Previous** and **Next** to move around.
- Filter by **Owner**, **Team** and **Type**. The team filter is available to people with `settings.manage`.
- Click an empty time slot to create a task at that time.
- Drag a task to another day or time to reschedule it, or use **Reschedule** on its menu. This only works for tasks you can update.
- A task's duration sets how long it looks on the grid.

A single view covers at most 62 days and 1000 entries. If it is truncated, a message asks you to use the filters.

## Notes

Notes are free text attached to a record. Open a person, organization or deal and use the **Notes** tab.

1. Select **Add note**.
2. Write in Markdown (bold, italic, lists, links). Use **Preview** to check it. Ctrl+Enter saves.
3. Choose **Visibility**: **Team** or **Private**.
4. Save.

| Item | Details |
| --- | --- |
| Length | Up to 50,000 characters. |
| Team notes | Seen by everyone who can read the record and has `notes.read`. |
| Private notes | Only the author sees them. Administrators cannot. Private notes never notify anyone. |
| Pin | Pinned notes are marked as pinned. Uses `notes.update`. |
| Edit and delete | The author can always edit and delete their own notes. For someone else's team note you need `notes.update` or `notes.delete` with a scope that covers the author. Only the author can change a note's visibility. |
| Mentions | A note can mention up to 50 active members, who are notified (team notes only). |

You can also add a note from the quick-add menu, choosing a record.

## Timeline

The **Timeline** tab of a record lists what happened to it, newest first: created and changed fields, stage changes, deal won or lost, tasks and activities, notes, files, and, when the matching modules are active, emails, calls and conversations. Use **Filter by type** to show only some event types and **Load more** for older items. You only see events of records you can read, and email events respect the conversation's visibility (see [Email and sequences](/docs/using/email-and-sequences/)).

## Calendar sync with Google and Microsoft

When the `google` or `microsoft` module is active and configured by an administrator, each user can connect their own calendar. Sync is for **meetings**:

- CRM tasks whose type is the meeting type (key `meeting` by default) and that you own are sent to your calendar. They need a due date. Duration defaults to 30 minutes when empty.
- Timed events in your calendar become meetings in Aphel. By default only events that include a person who is already a CRM contact are imported. An administrator can change this to import all timed events.
- When both sides change, the most recent change wins.
- Aphel looks 30 days back and 90 days ahead, syncs every 5 minutes, and pushes local changes about 15 seconds after you save.

To connect:

1. Open **Google** or **Microsoft** in the sidebar (under **Administration** in the menu).
2. Select **Connect calendar** and accept every permission the provider asks for.
3. The first sync starts straight away. Use **Sync now** to force another one, or **Disconnect** to stop. Meetings already created stay where they are.

The page shows the status (Connected, With errors or Access revoked), the last sync time and how many events were received and sent. If access is revoked, select **Reconnect calendar**.

You need the `google.calendar_sync` or `microsoft.calendar_sync` permission to connect. An administrator first has to enter the provider's Client ID and Client secret in the module settings. See [Integrations](/docs/admin/integrations/).

## Calls

The `telephony` module adds a **Calls** area, for teams that make many outbound calls. It uses the `tasks.*` permissions.

- **Sessions**: pick a saved view of people (up to 500) and start a **call session**. Aphel walks you through the people one by one: call, then **Log and next**, or **Skip**. An optional script is shown during the calls. Finish a session when done; pending people are left without a call.
- **Log call**: on a phone number, **Call and log the call** records the call as a task of type Call, with direction (inbound or outbound), outcome, duration and notes. The Call type must be active.
- **Outcomes**: administrators (`settings.manage`) define the outcomes (for example Answered). Each outcome can count as **connected**, which feeds the report. Keep at least one active.
- **Report**: shows calls by user and outcome for a period, with the connect rate and total duration. It lives in the **Insights** group of the menu.

## Things to know

- Deleting a task is permanent. Prefer completing it, or mark it Canceled through the API, if you want to keep history.
- If a record is permanently purged, tasks linked to it stay but lose the link.
- Fields hidden from your role by field permissions are not shown on tasks, and the Calendar and Inbox hide what you cannot see.
- A version conflict message ("Someone else changed this activity") means the task changed while you were editing. Reload and try again.
