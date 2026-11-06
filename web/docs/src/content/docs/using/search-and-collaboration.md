---
title: Search and collaboration
description: Find records fast with search and the command palette, and keep your team in sync with favorites, followers, tags, segments, mentions and notifications.
sidebar:
  order: 14
---

This page covers the features that help you find things and stay informed: global search and the command palette, favorites, following records, tags, segments, notifications, @mentions in notes and lead distribution.

## Availability

| Feature | Module | Availability |
| --- | --- | --- |
| Search and command palette | `search` | Recommended in the base CRM blueprint. The palette itself is always present; without the module it has no record results |
| Favorites | `favorites` | Recommended in the base CRM blueprint |
| Following records | `followers` | Recommended in the base CRM blueprint |
| Tags | `tagging` | Required by the base CRM blueprint |
| Segments | `segments` | Recommended in the Agency, SaaS and B2B Sales blueprints; optional elsewhere |
| Notifications | `notifications` | Recommended in the base CRM blueprint |
| Mentions | `notes` with `notifications` | Notes are required by the base CRM blueprint; the @ suggestion list comes from notifications |
| Lead distribution | `assignment` | Recommended in the Real Estate and B2B Sales blueprints; optional elsewhere |

Administrators switch modules on and off in [Modules](/docs/admin/modules/).

## Global search

Search looks across contacts, companies, deals, tasks and any other record type your workspace has, such as products.

**Open it.** Click the search box in the top bar or press `Ctrl+K` (`Cmd+K` on Mac) to open the command palette, then type. Results appear grouped by record type, up to five per group. Choose **See every result for "..."** to open the full **Search** page, which shows more results per type.

**What you can type.**

- At least 2 characters (and at most 100). Every word must match; matching is by part of a word and ignores case and accents.
- Names, email addresses, phone numbers and other searchable text, long text, email, phone and URL fields, including custom fields marked as searchable.
- Phone numbers are matched by their digits, so `11 99999-8888`, `999998888` and `+55 11 99999-8888` find the same number.

**What you will see.** You only get results for record types you can read (for example `people.read`, `deals.read`), limited to the records your permission scope allows (own, team or organization). Deleted and archived records and merged duplicates are left out, and fields hidden from your role are neither searched nor shown. Sensitive fields are never searched.

:::note
If a record you know exists does not show up, you may not have access to it, or an administrator may need to rebuild the search index.
:::

## The command palette

The palette is more than search. Open it and you can:

- **Go to** any page in your menu (for example "Go to Deals").
- **Create** a record. Press `C` for a create action based on the page you are on, or type what you want to create. Choose **Create new** from the create menu for the full list.
- Run actions for the page you are on, such as adding a note, opening a tab of the current record or copying its link.
- Open **Favorites**, add the current record to favorites or remove it.
- Open **Personal preferences** or **Show keyboard shortcuts**.

Use the arrow keys to move and `Enter` to run. The full list of shortcuts is in [Personal settings](/docs/using/personal-settings/#keyboard-shortcuts).

## Favorites

Favorites are your personal bookmarks, shown in the sidebar under **Favorites**.

- **Add a record.** Click the star next to the record title on its detail page (**Add to favorites**).
- **Pin a list view.** Click the pin on a saved view in a record list (**Pin view to favorites**).
- **Reorder.** Use the action menu of a favorite (**Move up**, **Move down**) or drag it.
- **Remove.** Use **Remove from favorites** in the menu or click the star again.

You can have up to 50 favorites. A favorite shows as **Unavailable item** when the record was deleted or you lost access to it.

## Following records

Following a record means you are told when something happens on it.

- Click **Follow** on the record page; click **Unfollow** to stop. The button shows how many people follow the record.
- The owner of a record follows it automatically when the record is created or when ownership changes to them.
- Open **Records I follow** (under **My work** in the menu) for a list of what you follow, filterable by type, with the date you started following. It shows up to 2,000 records.
- You are told about changes, new tasks, notes and emails on followed records, only for records you can read. You are never notified about your own actions. The notice is called **Records I follow** in your notification preferences and is in-app only until you turn on other channels.

## Tags

Tags label records so you can filter, segment and bulk-edit them.

- **Apply tags.** On a record page click **Tags**, search and tick the tags. You need the update permission of the record type (for example `people.update`). Tags also appear as a **Tags** column and filter in lists, and you can add or remove a tag on many records at once with the **Tags** bulk action (see [Data tools](/docs/using/data-tools/#bulk-actions)).
- **Manage tags.** Users with `settings.manage` open **Settings**, then **Tags**. Each tag has a name (up to 60 characters, unique), a color and the record types where it can be used; with nothing selected, it applies to all types. An organization can have up to 500 tags. The settings list shows how many records use each tag.
- Deleting a tag removes it from every record. Narrowing the record types of a tag keeps the tags already applied.

## Segments

A segment is a named, saved filter. Members are recalculated each time you open it and always respect what you are allowed to see.

1. Open **Segments** in the menu. It is available if you can read contacts, companies or deals.
2. Choose the record type.
3. Click **New segment**, enter a name and an optional description, and define the conditions. At least one complete condition is required.
4. Open a segment to see its records. A segment with a filter that is no longer valid shows **Invalid filter** until it is edited.

Only users with `settings.manage` can create, edit or delete segments. Others can view them. Names are unique per record type, a record type can have up to 200 segments, and a segment cannot reference another segment. Deleting a segment does not affect records. In record lists, a **Segment** filter lets you filter by a segment.

## Notifications

The bell in the top bar shows unread notifications. Open it to filter **All** or **Unread**, **Mark all as read**, or go to **Notification preferences**.

### Choosing what you receive

Open your account menu, then **My account**, then **Notifications**. A table lists each kind of notice with three channels:

- **In Aphel**: the bell.
- **Email**.
- **Push**: browser or phone notifications, even when Aphel is closed (see [Personal settings](/docs/using/personal-settings/#push-notifications)).

Tick or untick a box to change it; it saves immediately. A dash means the channel is not available for that notice. Preferences apply only to you.

| Notice | Default channels |
| --- | --- |
| Task assigned to me | In Aphel, email, push |
| Task due (within 1 hour, and at 8 am the day it is due) | In Aphel, email, push |
| Deal assigned to me | In Aphel, email, push |
| Stage change (someone else moves your deal) | In Aphel |
| Mention in a note | In Aphel, email, push |
| Reminder | In Aphel, email, push |
| Approval request, approval decided, approval overdue | In Aphel, email, push |
| Records I follow | In Aphel |
| Import finished or failed, export ready or failed, bulk action finished or failed | In Aphel |
| Automation notice | In Aphel |
| Automation email | Email |
| Automation stopped | In Aphel, email |
| Webhook disabled (only for people who manage settings) | In Aphel, email |
| Daily digest (tasks due today) | Email only, off by default |

Which notices appear depends on which modules are enabled. Times follow the organization time zone, and you are never notified about your own actions.

Read notifications are kept for 90 days and unread ones for 365 days.

## Mentions

In a team note you can mention a colleague so they are notified.

1. Open a note editor on a record (**Notes** tab, **Add note**).
2. Type `@` and start typing a name. Choose the person from the suggestions (up to six are shown).
3. Save the note. Each person mentioned receives a **Mention in a note** notice.

Rules to know:

- Only active members of the organization can be mentioned, and a note can mention at most 50 people.
- Private notes do not notify anyone. Visibility is **Team** or **Private**; only you can see a private note.
- Editing a note notifies only people newly added; people already mentioned are not notified again.
- If the person cannot read the record, they still get a notice, but without the record and with a message to ask you for access.

## Lead distribution

Lead distribution rules pick an owner for new records automatically. A user with `assignment.manage` creates them in **Settings**, under **Sales**, as **Lead distribution**.

| Strategy | Meaning |
| --- | --- |
| Round robin | Whoever waited longest gets the next record |
| Weighted round robin | Each member receives in proportion to their weight (1 to 100) |
| Least loaded | Whoever has the fewest active records of that type |

A rule has members (who can be marked away, with optional dates) and optional **routing by field** that sends a record to a team or users according to a field value, such as region or source. Use **Simulate** to preview who would receive the next records without saving anything, and **History** to see who received what. Rules can be used in automations, incoming webhooks, lead forms, bookings and imports (see [Data tools](/docs/using/data-tools/#relationships-and-owners)).
