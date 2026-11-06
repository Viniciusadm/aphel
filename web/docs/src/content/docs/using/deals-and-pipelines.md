---
title: Deals and pipelines
description: Track deals through pipelines, win and lose them, split credit across a deal team, forecast revenue, set goals and automate renewals.
sidebar:
  order: 6
---

A **deal** is a sale you are working on. Each deal sits in a **stage** of a **pipeline**, and moving it through the stages is how you record progress. This page covers deals, the Sales workflow that defines pipelines and stages, deal teams, items and recurring revenue, renewals, the forecast and goals.

## Availability

| Area | Module | Availability |
| --- | --- | --- |
| Deals, Kanban, deal team | `deals` | Required by the base CRM blueprint |
| Pipelines, stages, lost reasons, rotting | `workflow` | Recommended by the base CRM blueprint; Deals depends on it |
| Items on a deal | `line_items` | Agency, Consultancy, SaaS |
| MRR, ARR, TCV and contract dates | `deals_revenue` | Agency, Consultancy, SaaS |
| Automatic renewals | `renewals` | SaaS |
| Revenue forecast | `forecast` | SaaS, B2B Sales |
| Goals | `goals` | SaaS, B2B Sales |

Modules not recommended by your blueprint can still be enabled by an administrator; see [Modules](/docs/admin/modules/) and [Blueprints](/docs/admin/blueprints/). Leads convert into deals; see [Leads](/docs/using/leads/).

## The Deals page

Open **Deals** in the menu (or press `g` then `d`). The page has two display modes:

- **Kanban**: one column per active stage of a pipeline, with the number of deals and the total amount of each column. Drag a card to another column to move it. With the keyboard, press space or enter on a card, choose the stage with the arrow keys and press space or enter again. Columns load 20 cards at a time; use **Load more**.
- **Table**: the standard record list with search, filters, saved views and the archived filter; see [Records and views](/docs/using/records-and-views/).

If there is more than one pipeline, a selector lets you pick one or show all. Cards show how many days the deal has been in its current stage.

### Fields

| Field | Notes |
| --- | --- |
| Title | Required. |
| Amount | A non-negative number with up to 2 decimals. |
| Currency | Defaults to the organization currency. A converted amount is shown when an exchange rate exists, or "No rate" when it does not. |
| Pipeline, Stage | Where the deal is. |
| Status | **Open**, **Won** or **Lost**. Follows the stage; see below. |
| Probability (%) | Optional, 0 to 100. When empty, the probability of the stage is used. |
| Expected close date | Used by the forecast. |
| Contact, Company | The people and company involved. |
| Owner | Defaults to the user who creates the deal. Must be an active member. |
| Source | Up to 140 characters. |
| Lost reason | Set when the deal is lost. |
| Closed at, Stage entered at, Last activity | Read only. |

### Create a deal

1. Select **New deal**.
2. Fill in the title and, if you wish, amount, expected close date, contact, company and owner.
3. Save.

The deal starts in the first stage of the default pipeline. Needs `deals.create`.

## Win, lose, reopen and move

On a deal page the buttons are **Win**, **Lose**, **Reopen**, **Move stage** and a menu to move to another pipeline. All need `deals.update` on the deal.

- **Win** moves the deal to the first active won stage of its pipeline.
- **Lose** moves it to the first active lost stage and asks for a **Lost reason** (required) and an optional note of up to 5000 characters.
- **Reopen** returns a won or lost deal to an open stage and clears the closing date and the lost reason.
- Moving a deal into a won or lost stage by dragging or from **Move stage** has the same effect as Win or Lose.
- **Move to another pipeline** asks for the pipeline and the stage. If you do not choose a stage, the deal goes to the first active stage of the same kind.

Gotchas:

- The status follows the stage kind: open stages give **Open**, the won stage **Won** and the lost stage **Lost**.
- A stage can require fields. If a field required to leave the current stage or enter the new one is empty, a dialog asks you to fill it, then moves the deal. If you cannot see or edit that field, you are told to ask someone with access.
- A stage can have a required checklist. Required items must be checked before a deal advances to a later stage in the same pipeline or is won. Moving back, to another pipeline or losing the deal does not check it. Open the **Stage guidance** tab on the deal to read the guidance and tick items.
- An approval policy can block a move until someone approves it.
- Deals in pipelines restricted to teams you do not belong to are not visible to you.

## Archive, delete and the trash

- **Archive** hides the deal from lists; **Unarchive** brings it back. Needs `deals.archive`, granted to Admin, Manager and Member by default.
- **Delete** needs `deals.delete`. Deleted deals can be restored by an administrator from **Settings > Data > Trash**.

## Rotting deals

A stage can have a limit in days. An open deal that stays in the stage longer than that is **rotting**.

- Rotting deals show a warning badge, **Rotting for N days**, and a tinted row in the table.
- The **Rotting** toggle in the list toolbar shows only rotting deals.
- The limit is set per stage in the Sales workflow settings (**Days before rotting**, from 1 to 3650, empty for no limit; open stages only).
- Entering a new stage restarts the count.
- Automations have a trigger **Deal rotting in stage** that runs once per stage entry; see [Automations](/docs/using/automations/).

## Deal team

Open the **Team** tab of a deal to share the credit for the sale.

- Select **Add member**, choose a user, optionally a **Role**, and a **Credit (%)** from 0 to 100 with up to 2 decimals.
- The members together can take at most 100%. The owner keeps whatever is not given to members and is shown with the **Owner** badge.
- A deal can have at most 50 members. A user can be a member only once.
- Removing a member returns their credit to the owner.
- Adding a member notifies them ("Added to a deal team"), unless they turned that notification off.
- Members can see the deal even if their permission scope is "own records"; they can only view it, and editing follows the normal owner rule.

Changing the team needs `deals.update` on the deal. Administrators with `settings.manage` manage the available roles in **Settings > Deal team roles**. The base configuration installs **Salesperson**, **Pre-sales**, **Specialist** and **Manager**; you can add, rename, reorder, deactivate and delete roles (up to 50). Deleting a role leaves its members without a role and keeps their credit.

Team credit is used by goals with the split credit mode.

## Sales workflow: pipelines, stages and lost reasons

Administrators with `settings.manage` open **Settings > Sales workflow** (under **Fields and records**).

### Pipelines

- Create a pipeline with **New pipeline**. It starts with six default stages (Start 10%, Review 25%, Preparation 50%, In progress 75%, Completed, Cancelled) that you can edit. The base CRM configuration installs one pipeline called **Default workflow**; other blueprints install their own, for example B2B Sales has Prospecting, Qualification, Demo, Proposal, Negotiation, Won and Lost.
- The **default** pipeline opens first on the Kanban and receives deals created without a pipeline. The default pipeline cannot be deactivated or deleted, and an inactive pipeline cannot be the default.
- **Deactivate** a pipeline to stop creating or moving deals into it; its deals stay where they are.
- **Deleting** a pipeline moves its deals to another pipeline you choose, mapping each stage to a target stage.
- **Teams with access**: restrict a pipeline to some teams. With no team selected, everyone sees it. Administrators always see every pipeline.
- You can have up to 50 pipelines, with up to 50 stages each. Reorder pipelines with the move buttons.

### Stages

Each stage has a **Name**, a **Probability (%)** from 0 to 100 and a **Kind**: **In progress** (open), **Won** or **Lost**.

- A pipeline must keep at least one active open, won and lost stage.
- The kind of a stage cannot change while it has deals.
- A stage with deals cannot be deactivated or deleted until you choose a stage of the same kind to receive them.
- Stages can be deactivated and reactivated.
- **Fields required to enter** and **Fields required to leave** make a deal unmovable until those fields are filled.
- **Days before rotting** is described above.
- **Stage guidance** holds markdown guidance and a checklist; mark items **Required** to enforce them when advancing.

### Lost reasons

Lost reasons are offered when a deal is lost. You can add, rename, reorder, deactivate and reactivate them, up to 100, but not delete them. At least one must stay active. The base configuration installs **Requirements not met**, **Alternative selected**, **Resources unavailable**, **No response**, **Timing** and **Other**.

## Items on a deal

With the `line_items` module, deals get an **Items** tab. Add items from the product catalog or as free items, with quantity, unit price, discount (none, percentage or amount), tax and, for recurring sales, a billing frequency (one-time, monthly, quarterly, yearly) and a term in months (1 to 600). A deal can have up to 200 items. Reorder items with the arrows. You need `deals.update` to change them and `line_items.read` to see them.

**Settings > Products and items** sets how the deal amount relates to items:

- **Manual** (default): you type the amount; items are informative.
- **Sum of items**: deals that have items get the amount from the sum and the Amount field becomes read only. Deals without items stay manual. Current amounts are not rewritten when you switch the mode; each deal follows the sum at its next item change, or when you select **Use total as amount**.

A deal with items cannot change currency; remove the items first. Products must be in the same currency as the deal.

## Recurring revenue

With `deals_revenue`, the **Revenue** tab of a deal shows figures calculated from items that have a billing frequency:

- **MRR**: the monthly equivalent of all recurring items.
- **ARR**: MRR times 12.
- **Total contract value (TCV)**: one-time items plus each recurring item over its term. An item without a term counts as 12 months.
- **Contract start** and **Contract end**: the start is set when the deal is won unless you filled it in. The end is the start plus the longest item term, minus one day.

A dashboard widget, **Won recurring revenue**, totals these for won deals.

## Renewals

With the `renewals` module, an administrator (`settings.manage`) can have Aphel create renewal deals. In **Settings > Sales workflow**, select the pipeline and use the **Automatic renewal** section:

- **Create renewals automatically**: on or off. Off by default.
- **Days before the contract ends**: 0 to 365, 30 by default.
- **Renewal pipeline** and **Renewal stage**: where the renewal deal is created. The stage must be an active open stage.
- **Title prefix**: up to 60 characters, placed before the original title.
- **Owner**: the same as the original deal, or no owner.

An hourly job looks at the won deals of the pipeline whose contract ends within the chosen number of days. For each one it creates a renewal deal with the same contact, company and currency, a copy of the recurring items, a contract start the day after the original ends, and a **Renewal of** link to the original. Each deal is renewed only once. The renewal is created with the permissions of whoever saved the settings last. A **Renewal created** notification kind exists in your notification preferences, and the **Contracts up for renewal** dashboard widget lists what is coming.

## Revenue forecast

With the `forecast` module, **Forecast** in the menu (needs `deals.read`) estimates revenue per month from open deals, grouped by expected close date, from the current month through the horizon.

For each month it shows the open amount, the forecast and the amount won (by closing date). Select a month to see its deals, largest first.

An administrator with `settings.manage` chooses the calculation in **Settings > Revenue forecast** (the **Configure** link on the page):

| Method | Forecast is |
| --- | --- |
| Stage probability | Amount times the probability of the deal's stage. |
| Deal probability (default) | Amount times the probability set on the deal, or of the stage when empty. |
| Commit only | The amount of deals in the commit category. |

Optionally split the forecast into categories, by the probability used: **Pipeline** (below best case), **Best case** (from 50% by default) and **Commit** (from 90% by default). Commit must start above best case. The default horizon is 6 months; the maximum is 18.

The forecast follows your permissions: it counts only deals you can read. If you cannot see the Amount or Expected close date fields, it is unavailable; if you cannot see the deal probability, the stage probability is used.

## Goals

With the `goals` module, **Goals** in the menu tracks targets for a user, a team or the whole organization.

A goal has an owner, a **Metric**, a **Period** and a target:

| Metric | Counts |
| --- | --- |
| Amount won | Value of deals won in the period (by closing date). |
| Deals won | Number of deals won in the period. |
| Activities completed | Activities completed in the period. |
| New people | Contacts created in the period. |

Periods are **Month**, **Quarter** or **Year**. Count metrics take whole numbers. Only one goal can exist for the same owner, metric and period.

Progress is computed from the records the viewer can see, and shows as **Not started**, **On track**, **Behind**, **Achieved** or **Missed**. A goal is achieved as soon as the target is reached. Before that it is on track when the progress is at least the share of the period already elapsed, behind otherwise, and missed once the period ends without reaching the target. Where you lack access to the underlying data, progress shows as unavailable.

For **Amount won** goals of users and teams, **Revenue credit** can be **Whole amount for the owner** (default) or **Split by the deal team**, which adds each user's share of the won deals according to the deal team credit.

Permissions: everyone sees goals for themselves, their teams and the organization. Creating and editing needs `goals.manage`, whose scope decides for whom: own goals only, your teams and their members, or anyone and the organization. Goals can also be shown on the dashboard with the **Goals progress** widget.

## Reports

The Reports area includes a **Conversion** report (funnel, time per stage, where and why deals are lost) and a **Performance** report (results grouped by salesperson, team, source or pipeline). They need `deals.read` and follow your permissions. See [Dashboards and reports](/docs/using/dashboards-and-reports/).

## Permissions

| Permission | What it allows |
| --- | --- |
| `deals.read` | See deals, the Kanban, the Forecast and sales reports. |
| `deals.create` | Create deals; convert leads. |
| `deals.update` | Edit, move, win, lose and reopen deals; manage the deal team and items. |
| `deals.archive` | Archive and unarchive. |
| `deals.delete` | Delete. |
| `deals.export` | Export deals. Not granted to the Member role by default. |
| `workflow.read` | Read pipelines. Granted to Admin, Manager and Member. |
| `line_items.read` | See items. Granted to Admin, Manager and Member. |
| `goals.manage` | Create and edit goals, within its scope. |
| `settings.manage` | Sales workflow, renewals, deal team roles, forecast settings, products and items settings. |

Record permissions have a scope (own, team or organization); see [Roles and permissions](/docs/admin/roles-and-permissions/).
