---
title: Records and views
description: How to work with record lists, filters, saved views, calendar layouts, record pages, sharing, archiving and deleting in Aphel.
sidebar:
  order: 2
---

A record is one item of a type: a contact, a company, a deal, a task, a product, a quote or an item of a custom object. Every record type uses the same list, filter, form and record page building blocks, so what you learn on Contacts applies to Companies, Deals and custom objects.

## Permissions that gate records

Each record type has a family of permissions named `<type>.<action>`. Contacts use `people.*`, companies `organizations.*`, deals `deals.*`, and a custom object `custom.<key>.*`.

| Permission | What it allows |
| --- | --- |
| `<type>.read` | See the type in the menu, open lists and record pages. |
| `<type>.create` | Create records. |
| `<type>.update` | Edit records. |
| `<type>.archive` | Archive and unarchive records. |
| `<type>.delete` | Delete records. |
| `<type>.export` | Export records. |

Roles can also limit how far a permission reaches (for example your own records, your team's records or the whole organization). If a list looks shorter than a colleague's, the read scope of your role is the usual reason. See [Roles and permissions](/docs/admin/roles-and-permissions/).

## Lists

A list shows records of one type as a table. The toolbar, from left to right, holds the view switcher, a search box, **Filters**, optional layout controls, **Display** and the page actions.

### Search

The search box filters the list by text. It looks at the fields marked **Searchable** in the field settings. See [Custom fields](/docs/using/custom-fields/). To search across all record types at once, use the command palette (Ctrl+K or Cmd+K) or the Search page, described in [Search and collaboration](/docs/using/search-and-collaboration/).

### Sorting and paging

Click a column header to sort ascending, click again for descending. Hold Shift while clicking to combine sorts; up to three sort rules are kept. Multi-select, long text, user and related-record fields cannot be sorted.

Under **Display** you can choose the page size (25, 50 or 100 records, 25 by default).

### Columns and density

Open **Display** to change how the table looks:

- **Columns**: show or hide a column, drag a column to reorder it, or choose **Reset to default**.
- **Density**: **Compact** or **Comfortable**.
- **Archived** (on Contacts, Companies, Deals and custom objects): **Hide archived** (default), **Include archived** or **Archived only**.

Column, density and sorting changes are temporary until you save them in a view (see below). Anyone can change them for the current visit without affecting other people.

### Filters

Select **Filters** to build conditions.

1. Choose **Add condition**.
2. Pick a field, an operator and a value.
3. Combine conditions with **All conditions (AND)** or **Any condition (OR)**.
4. Use **Add group** to nest a group of conditions with its own AND/OR logic.
5. Select **Apply filters**. Active filters appear as removable chips under the toolbar; **Clear filters** removes them all.

Incomplete conditions are ignored. A filter can hold up to 50 conditions.

The operators depend on the field type:

| Field type | Operators |
| --- | --- |
| Text, link, email, phone | is, is not, contains, does not contain, starts with, is any of, is none of, is empty, is not empty |
| Long text | contains, does not contain, is empty, is not empty |
| Number, currency | is, is not, greater than, greater than or equal to, less than, less than or equal to, between, is any of, is none of, is empty, is not empty |
| Date, date and time | is on, is not on, after, on or after, before, on or before, between, plus the relative operators below |
| Single select, user, related record | is, is not, is any of, is none of, is empty, is not empty |
| Multi select | is any of, is none of, is empty, is not empty |
| Checkbox | is, is not |

Relative date operators are: is today, was yesterday, is tomorrow, this week, last week, next week, this month, last month, next month, this year, in the last N days, in the next N days, more than N days ago, anniversary in the next N days, and is empty.

For user fields you can pick **Me (current user)**, so a saved filter works for whoever opens it. A related-record field that points to a hierarchical type also offers "belongs to the group of".

**Conditions on related records.** Choose **Add related condition** to filter by something stored on a linked record. Pick the relation, then add conditions on that record's fields. The list then shows records that have at least one matching related record, or, with **No related record matches**, records that have none.

Fields that are hidden from your role are not available in filters and cannot be used to sort. Filtering by them is refused.

## Saved views

A saved view stores the current filters, sorting, columns and density, plus the calendar layout when you use it. Views belong to a record type and are listed in the **View** menu at the left of the toolbar. **All records** is the unfiltered list.

When you change filters, sorting or columns on top of a view, the view shows a **modified** badge. Nothing is stored until you choose what to do:

| Action | Effect |
| --- | --- |
| **Save changes** | Overwrites the active view with the current setup. Shown only if you may edit that view. |
| **Save as new view** | Opens the **Save view** dialog to create another view from the current setup. |
| **Discard changes** | Returns to the stored view. |

### Create a view

1. Set up the list the way you want it.
2. Open the **View** menu and choose **Save as new view**.
3. Enter a **Name** (1 to 140 characters, for example "My hot leads").
4. Choose **Who can see** and select **Save**.

### Who can see a view

| Option | Who sees it |
| --- | --- |
| Only me | Only you. |
| My team | Members of the team you pick. You can only share with a team you belong to, unless you have `settings.manage`. |
| Entire organization | Everyone with read access to the record type. Only users with `settings.manage` can create or keep organization views. |

### Edit, rename and delete

With a view active, the View menu shows its name followed by the actions available to you: **Edit view** (name and visibility), **Delete**, and **Set as my default** or **Remove as my default**.

- The owner can change or delete a private or team view.
- An organization view can only be changed or deleted by users with `settings.manage`.
- Users with `settings.manage` can edit and delete any view they can see.
- Deleting a view removes it for everyone who uses it and cannot be undone.

Edit and Delete do not appear when you are not allowed to use them. A view can hold at most 100 columns, and every column must be a field you can see.

### Default views

When you open a list without a link or filters, Aphel opens a default view if one exists:

1. **Your default**, set through **Set as my default**. Any view you can see can be your default.
2. Otherwise the **Organization default**, a view with visibility "Entire organization" marked **Use as organization default**. Only users with `settings.manage` can mark one, and only one view per record type holds that mark at a time.

If you open a link to a view that was deleted or that you cannot see, the list shows a notice that the linked view no longer exists or you do not have access to it.

Views can also arrive through configuration packages, so a new workspace often starts with a few ready-made views. See [Configuration packages](/docs/admin/config-packages/).

### Sharing a view by link

The address of a list carries the active view, filters, sorting, search and page. Copy the address from the browser to hand someone the exact same list. They still only see records their role allows.

## Calendar layout

Record types that have at least one filterable date or date-and-time field offer a **Table / Calendar** switch in the toolbar. Tasks and deals, for example, can be shown by due date or expected close date.

- Choose **Calendar**, then pick the **Calendar date field**. Aphel prefers `due_at` or `expected_close_date` when they exist, otherwise the first date field.
- Switch between **Month** and **Week**, and move with **Previous**, **Next** and **Today**.
- Drag a record to another day to change its date. This is available when the date field is editable for you. For date-and-time fields the time of day is kept. If the update fails, the record returns to its day.
- The calendar loads at most 500 records per period. When there are more, a notice reads "Showing N of M records in this period. Refine the filters to see the rest."
- Filters, search and the archived option apply to the calendar too. Page size, columns and density apply only to the table.
- Saving a view while in Calendar layout stores the layout and the date field, so the view reopens as a calendar.

A separate calendar for tasks, meetings and connected calendars is described in [Tasks and calendar](/docs/using/tasks-and-calendar/).

## Board layout

Some lists offer a **Kanban** board instead of a table. Deals open as a board by default, grouped by pipeline stage, with a **Table** option next to it. Custom objects that have single-select fields offer a **Display mode** with **List** and **Kanban**, where you choose the **Group by** field. Views saved from a board remember the board layout. Pipelines and stages are covered in [Deals and pipelines](/docs/using/deals-and-pipelines/).

## Selecting records

Tick the box in a row to select it, or the header box to select the current page. When a page is selected, **Select all N matching records** extends the selection to every record that matches your filters, not just the page. **Clear selection** resets it. Changing the filters, search, view or archived mode clears the selection.

With a selection, the toolbar offers the bulk actions that your enabled modules provide. See [Data tools](/docs/using/data-tools/).

## Creating and editing records

**New** (for example **New deal**) opens a form dialog with the fields of that type. Required fields are marked **required**. The dialog offers **Continue in full form** to move to a full-page form when you need more room. The form follows the layout your administrator defined for creating records. Layouts, conditional visibility and conditional required rules are described in [Custom fields](/docs/using/custom-fields/#record-layouts).

On a record page, select **Edit** for the full form, or click a field value to edit it in place when the field is editable. Leaving a form with unsaved changes asks whether to **Keep editing** or **Leave without saving**.

When a save fails, the message names the fields to fix. The most common reasons are:

- a required field is empty;
- the value does not match the field rules (range, length, pattern, decimals, format);
- the value is not unique;
- a field is read-only or hidden for your role;
- a validation rule of your organization blocks the change;
- someone else changed the record since you opened it. Reload the page and apply your change again.

## Record pages

A record page has a header and a body.

The header shows the title, badges, the **Owner**, a back link, the main action and a **More** actions menu (named **Actions**). Archive, delete and other actions of the type live there. The **Share** button sits in the header of the standard record pages.

The body shows the record's fields and a set of tabs. Tabs come from the modules you have enabled. The standard ones are:

| Tab | Content |
| --- | --- |
| Timeline | The record's history and activity, with a filter by event type. |
| Tasks | Tasks and activities linked to the record. |
| Notes | Notes written on the record. |
| Files | Attachments. |
| Related | Linked records. |

Other modules add their own: for example Revenue and Team on deals, Proposals, Emails, Sequences, Messaging conversations, or AI assistant. A tab only appears when its module is enabled and you have the permission it needs. Records of a custom object get a **Relationships** tab when the record has links. On phones, tabs are grouped under Activity, Relationships, Communication, Insights and More, behind a **Sections** control.

If one section fails to load, the page shows a message for that section and the rest keeps working.

The layout of the fields panel, including tabs, sections and columns of the view, is controlled by the "View" form layout of the record type. See [Custom fields](/docs/using/custom-fields/#record-layouts).

## Sharing a record

Sharing gives a person or a team access to one record beyond what their role allows.

1. Open the record and select **Share**.
2. Under **With**, choose **User** or **Team**, then pick who.
3. Choose the **Access**: **Read** or **Edit**.
4. Select **Share**.

To change the access of an existing share, use the access selector in its row. To stop sharing, choose **Remove share**.

| Access | Effect |
| --- | --- |
| Read | The person sees the record in lists, search, reports and exports. |
| Edit | The person can also edit, move and archive it, but only when their role allows editing that record type. |

Rules to know:

- Sharing never allows deleting the record or sharing it again.
- Only the record's owner, or someone with `records.share`, can add, change or remove shares. Everyone else sees the list read-only.
- You cannot share a record with its owner. A record can have at most 100 shares, and the same user or team cannot be added twice.
- Shared access does not widen what a role forbids in general: Edit needs `<type>.update` or `<type>.archive` in the person's role, and Read needs `<type>.read` or `<type>.export`.

## Archive, delete and restore

Aphel keeps three states apart.

**Archive.** An archived record is hidden from lists but keeps all its data and links. Use it for records you no longer work on but want to keep. On the record page choose **Archive** (needs `<type>.archive`); an **Archived** badge marks it, and **Unarchive** brings it back. To find archived records in a list, open **Display** and choose **Include archived** or **Archived only**.

**Delete.** On the record page choose **Delete** (needs `<type>.delete`) and confirm. For contacts and companies you must type the record's name to confirm. Deleted records leave your lists and go to the Trash. A delete can be blocked when other records are linked to the record in a way that forbids deletion, for example a required relationship of a custom object. Remove the links first.

**Trash.** The Trash lives under **Data > Trash** in Settings and needs `settings.manage`. Deleted records of contacts, companies, deals, products, tasks, quotes and custom objects stay there for a set number of days. The page lists:

| Column | Meaning |
| --- | --- |
| Record | The deleted record. |
| Type | The record type. |
| Deleted on | When it was deleted. |
| Permanent deletion | When a daily routine will erase it. |

Use the type filter or **Search the trash** to find a record, then:

- **Restore** brings the record back with its links and custom fields. Restoring goes through the same checks as a normal save, so a validation rule or a pending approval can block it.
- **Delete permanently** erases the record's values, notes, attachments and tags. It cannot be undone.

You only see trashed records you could read, and restoring or purging a record also needs delete permission on it. The retention period is set in the same page under **Time in the trash**: between 1 and 365 days, 30 by default.

:::caution
The confirmation text when deleting a contact says the record can no longer be undone from the interface. If the Trash module is enabled, an administrator with `settings.manage` can still restore it from **Data > Trash** until the retention period ends.
:::

## Tips

:::tip
Build a few views for the questions you ask every day, such as "My open deals closing this month" with a filter on **Owner is Me**, and set one as your default. Share the ones your team needs with **My team**.
:::

- A hidden or read-only field in a form is a field permission set by your administrator, not a bug. See [Custom fields](/docs/using/custom-fields/#field-permissions).
- Selecting **Save changes** is only offered on views you can edit. On a view you cannot edit, use **Save as new view** to keep your version.
