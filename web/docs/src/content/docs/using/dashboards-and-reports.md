---
title: Dashboards and reports
description: Build your home dashboard from widgets, choose the period and whose results to see, and create, share, export and schedule saved reports.
sidebar:
  order: 9
---

The **Home** screen is a dashboard made of widgets. **Reports** lets you build your own tables and charts from your records, share them, export them and email them on a schedule.

## Availability

| Area | Module | Notes |
| --- | --- | --- |
| Dashboard and the widget engine | `dashboard` | On by default; recommended in every blueprint. |
| Saved reports | `reports` | Recommended in Agency, Consultancy, Other business and B2B Sales. |
| Goals widget and Goals page | `goals` | Recommended in B2B Sales and SaaS. |
| Forecast page | `forecast` | Recommended in B2B Sales and SaaS. It has a page of its own, not a widget. |
| Renewals and recurring revenue widgets | `renewals`, `deals_revenue` | Recommended in SaaS (`deals_revenue` also in Agency). |

Widgets are contributed by modules, so which ones you can add depends on the active modules and your permissions. See [Modules](/_aphel/docs/admin/modules/).

## The dashboard

Open **Home** under **My work**. Every user has their own layout. Until you change it, you see the organization default, or, if none was configured, the built-in default. Administrators can set the organization default through configuration packages (see [Configuration packages](/_aphel/docs/admin/config-packages/)).

### Period

The **Period** selector at the top applies to every widget that is based on a period. The default is **This month**. Choose a preset (Today, Yesterday, Last 7 days, Last 30 days, This week, Last week, This month, Last month, This quarter, Last quarter, This year, Last year) or **Custom** with a start and end date. A custom range can span at most 731 days, and the start must be before the end. Dates use the organization time zone.

Widgets that do not depend on a period (for example Pending work, Recent activity, Goals) ignore it.

### Whose results

**Results of** changes whose numbers the period-based widgets show. The options depend on how much of the data your role can see:

- **My results**, **My teams** or **Whole organization** (the broadest one your role allows),
- a specific team or person from the lists shown.

You only see teams and people your role allows. **Compare with the previous period** adds the previous period's numbers next to the current ones for widgets that use a period.

### Customize the layout

1. Select **Customize**.
2. Use **Add widget** to pick from the widgets available to you, grouped by category. A widget already on the dashboard is not offered again.
3. On each widget you can drag it to reorder, resize it (**Small**, **Medium**, **Large**), open its settings, or remove it.
4. Select **Done**.

Changes are saved automatically for you only. **Restore default** (while customizing) discards your layout and goes back to the organization default. A dashboard can hold up to 24 widgets.

If a widget fails to load, only that widget shows an error. A widget your role can no longer use is hidden.

### Widgets

| Widget | Needs | Shows | Settings |
| --- | --- | --- | --- |
| Pending work | `tasks.read` or `deals.read` | Overdue tasks, tasks due today and tomorrow, and open deals without recent updates. A section is hidden when you lack access to it. | Days without update (1 to 365, default 14), Only mine (default on; ignored when you pick another person or team under Results of). Up to 5 items per list by default. |
| Summary | One of `deals.read`, `tasks.read`, `people.read` | Open, won and lost deals; pending tasks and activities done in the period; number of contacts. | None |
| Recent activity | Read access to any record type | The latest events on records you can read: who did what, and where. | Number of items (5 to 50, default 15) |
| Sales indicators | `deals.read` | Open deals, pipeline value, forecast revenue (value times probability), deals won and lost in the period and conversion rate. Amounts are hidden if your role cannot see them. | None |
| Deals by stage | `deals.read` | Count and value of deals in each stage of the pipeline. | None in the web app |
| Goals | A goal you can see | Progress of goals in the current period. | Number of goals (1 to 20, default 5), Only my goals |
| Saved report | Read access to the report's data | A saved report as a table or chart. | Use the dashboard period |
| Deals by source | `deals.read` | Deals created in the period grouped by their Source field. | Sources shown (3 to 20), Won deals only |
| Team activity ranking | `tasks.read` | Tasks completed in the period by owner. | People in the ranking (1 to 20), Split by activity type |
| This month's birthdays | `people.read` | Contacts whose birthday is this month. | Contacts shown (5 to 50) |
| Contracts up for renewal | `deals.read` | Contracts ending within a window and their MRR. | Window in days, Only mine, Items in the list |
| Won recurring revenue | `deals.read` | Won MRR, ARR and TCV in the period. | None |

Which module provides each widget is in the first table above. Plugins can also add widgets.

The birthdays widget uses the **Birthday** field of contacts. Fill it in on each contact; if an administrator deactivates the field, the widget says it is unavailable.

:::note
Deal amounts, and any field hidden from your role by field permissions, are left out of widgets and reports for you.
:::

## Goals and forecast

- **Goals** (under **Insights** in the menu) tracks goals per user, team or organization for a month, quarter or year, with metrics such as Amount won, Deals won, Activities completed and New people. Goal statuses are Not started, On track, Behind, Achieved and Missed. Creating and editing goals requires `goals.manage`. The Goals widget shows the current period's goals on your dashboard.
- **Revenue forecast** (under **Insights**, needs `deals.read`) gives a monthly estimate from the amount, probability and expected close date of open deals, by Pipeline, Best case and Commit. Administrators choose the calculation method in settings under **Sales**, **Revenue forecast**. The dashboard has no forecast widget; **Sales indicators** shows a simpler forecast figure.

## Reports

Open **Reports** (under **Insights**). The Report library shows **My reports** and any report pages other modules add, such as the Calls report.

### Build a report

1. Select **My reports**, then **New report**.
2. In **Details**, give it a name (up to 140 characters) and optional description, and choose **Sharing**: **Only me**, **My team** (then choose a team) or **Whole organization**.
3. Choose the **Module** (the kind of record to analyze, among those you can read).
4. Add up to 3 **Groups** (fields to group by; for dates choose a granularity: day, week, month, quarter or year).
5. Add **Metrics**: count of records, sum, average, minimum or maximum of a numeric field, or the duration between two date fields in days or hours.
6. Optionally add **Filters**, limit by **Period** on a date field (with an option to compare with the previous period), set **Sorting** and a maximum number of rows.
7. Pick a **Visualization**: Table, Bars, Lines, Pie or Number. The **Preview** updates as you go.
8. Select **Save**.

Reports are always computed with the permissions of whoever is looking at them, so two people can see different numbers from the same report. Sharing needs the right to share with that audience: anyone can keep a report private, only members of a team can share with that team, and sharing with the whole organization requires `settings.manage`.

### Edit, export and delete

- The owner can edit a report that is private or shared with a team. A report shared with the whole organization can only be edited by someone with `settings.manage`. Others can view and export it.
- **Export** downloads a spreadsheet (**XLSX**) or **CSV** (semicolon-separated).
- **Delete** removes the report and its email schedules. Dashboard widgets using it stop showing data.

### Put a report on the dashboard

On a saved report, select **Add to dashboard**. It is added as a large Saved report widget. In the widget's settings, **Use the dashboard period** decides whether the dashboard period replaces the period saved in the report. You can add the same or different reports several times this way.

### Email a report on a schedule

On a saved report, use **Schedule delivery**.

| Option | Details |
| --- | --- |
| Frequency | Daily, Weekly (choose weekday) or Monthly (choose day of month) |
| Time | In the organization time zone |
| Delivery period | Same as the report, or a different preset |
| Recipients | Up to 50 active members who can see the report |
| Delivery active | Turn off to pause |

Each recipient receives the numbers computed with their own permissions, with a link to the report and an XLSX attachment when it is 5 MB or smaller. A report can have at most 20 schedules. Use **Show latest deliveries** to see whether each was Sent, Sending or Failed. Delivery is checked every 5 minutes. Emails are sent by the system's **Outgoing email** configuration, not by anyone's personal mailbox, so an administrator must have set that up (see [Configuration](/_aphel/docs/admin/configuration/)).

## Where numbers come from

- Widgets and reports read live data; there is no nightly cache. Each widget shows its own error if it cannot load.
- A widget that needs a module you do not have, or a permission you lack, is simply not offered.
- If numbers differ from a colleague's, compare your role scope (own, team, organization) and field permissions first. See [Roles and permissions](/_aphel/docs/admin/roles-and-permissions/).
