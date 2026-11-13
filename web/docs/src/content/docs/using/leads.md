---
title: Leads
description: Work the lead inbox, qualify, disqualify and convert leads, and configure lifecycle rules, lead scoring and lead distribution.
sidebar:
  order: 5
---

A lead is a contact that has not yet become an opportunity. Aphel does not keep leads in a separate table: a lead is a contact whose **Lifecycle stage** is **Lead**. This page covers the lead inbox, the lifecycle stages, scoring, automatic assignment of owners and the optional Lead Pro plugin.

## Availability

| Area | Module | Recommended in blueprints |
| --- | --- | --- |
| Lifecycle stages, lead inbox, qualify, disqualify, convert | `leads` | Agency, Other business, Real Estate, SaaS, B2B Sales |
| Lead scoring | `scoring` | SaaS, B2B Sales |
| Lead distribution | `assignment` | Real Estate, B2B Sales |
| Hot leads and lead temperature | `lead_pro` | None. Optional plugin outside the default build |

The base CRM blueprint recommends none of these, and the Freelancer and Consultancy blueprints do not list them either. An administrator can enable any of the modules on the Modules page; see [Modules](/_aphel/docs/admin/modules/) and [Blueprints](/_aphel/docs/admin/blueprints/).

## Lifecycle stages

The **Lifecycle stage** field exists on contacts and on companies. The stages are:

| Stage | Meaning |
| --- | --- |
| Lead | A contact not yet qualified. These are the contacts shown in the lead inbox. |
| Qualified | A lead that was reviewed and is worth pursuing. |
| Opportunity | The contact has a deal in progress. |
| Customer | The contact has a won deal. |
| Former customer | A customer that is no longer active. |

A new contact has no stage unless one is set. You can set the stage by hand on the contact. Contacts created automatically from synchronized email start as **Lead**, and a form can be configured to set the stage. Every change is recorded: open the **Lifecycle** tab of a contact or company to see each stage, who or what changed it (a person or **automatic rule**), when it was entered and left, and how many days it stayed there.

Only the active stage options can be chosen. The list of stages is a normal select field; administrators edit its options in the field settings (see [Custom fields](/_aphel/docs/using/custom-fields/)).

## The lead inbox

Open **Leads** in the menu. Leads and Contacts are siblings: both show the same people, and Leads is the work queue for those who have not become a deal yet. The page has three tabs with counts: **New** (Lead stage), **Qualified** and **All open**. Columns start with source, owner and entry date (and score when scoring is enabled). Use **New lead** to add a contact already in the Lead stage, and **All contacts** to go to the full list. A **Leads only** quick filter is also available on the Contacts list.

For each lead you can:

- **Qualify**: moves the lead to **Qualified**, so it leaves the **New** tab. Only leads in the Lead stage can be qualified.
- **Disqualify**: opens a dialog where you choose a **Reason** and optionally add a **Note** (up to 2000 characters). The lead leaves the inbox. The reason and note are kept in the history and feed the conversion report.
- **Convert**: creates a deal for the lead (see below).

The same buttons appear on the contact page for contacts in the **Lead** or **Qualified** stage. Qualifying a lead moves it to the **Qualified** tab, where you can still disqualify or convert it.

Permissions:

- Qualify, disqualify and convert need `people.update` on the contact. Without it the buttons are hidden.
- Convert also needs `deals.create`.
- Opening the Leads page needs `people.read`.

### Disqualification reasons

The reasons offered in the dialog are managed by administrators with `lifecycle.manage` in **Settings > Lifecycle**. The base CRM configuration installs six: **No budget**, **Not a fit**, **Unresponsive**, **Chose a competitor**, **Invalid contact details** and **Other**.

- You can add, rename, reorder, deactivate and delete reasons. There can be at most 50.
- A deactivated or deleted reason stays visible on past disqualifications but cannot be picked again.
- Names have up to 140 characters. The key of a reason never changes; configuration packages use it.

In the same settings page, **Stage of a disqualified lead** chooses which lifecycle stage a disqualified contact receives. By default it is no stage, which clears the stage.

### Convert a lead into a deal

1. Select **Convert** on the lead.
2. Choose the **Pipeline** and, optionally, the **Stage**. By default the deal starts in the first stage of the pipeline.
3. Optionally change the **Deal title** (it defaults to the contact name) and enter an **Amount**.
4. Select **Convert**.

Aphel creates the deal with the contact, the contact's primary company and the contact's source, then moves the contact to the target stage of the "Deal created" rule (**Opportunity** by default). The result is subject to the deal's usual rules: required stage fields, validation rules and approvals can block it. A lead without a primary company is converted without one.

## Automatic stage transitions

In **Settings > Lifecycle**, the **Automatic transitions** section changes the stage of a contact and of its company when deals change. Each rule has an on/off switch, a target stage and the stages it applies from. A rule only changes records in the stages you check, so it never demotes a customer unless you ask for it.

| Trigger | Default target | Default applies from |
| --- | --- | --- |
| Deal created or reopened | Opportunity | Lead, Qualified, Former customer or no stage |
| Deal won | Customer | Lead, Qualified, Opportunity, Former customer or no stage |
| All deals lost | Qualified | Opportunity only |

"All deals lost" fires when the last open deal is lost or deleted and the record has no won deal. The changes are applied in the background, a moment after the deal changes, and show up in the stage history as made by an automatic rule. You need `lifecycle.manage`, which Admins have by default.

## Conversion report

From the lead inbox, select **Conversion report**. For contacts or companies, over a period (the last 90 days by default, up to 731 days), it shows for each stage how many records are there now, how many entered the period, the conversion to later stages, and the average time spent. For contacts it also lists disqualifications by reason. It counts only records you can see.

## Lead scoring

Scoring ranks records as **Cold**, **Warm** or **Hot** by adding points for their profile and behavior. Administrators with `scoring.manage` manage models in **Settings > Lead scoring**.

A model belongs to one record type (contacts, companies or deals) and has:

- **Bands**: Warm from and Hot from. Below Warm the score is Cold. Defaults are 30 and 70.
- **Profile criteria**: points added while the record matches a filter, in the same filter format as list views. Points can be negative.
- **Behavior criteria**: points added for each occurrence of a signal: **E-mail opened**, **E-mail link clicked**, **E-mail replied**, **Meeting booked**, **Form submitted** or **Call answered**. You can set a **Points cap** and **Decay (days)**: each occurrence loses its points steadily until it reaches zero after that many days. Signals of a person also count for the models of their company.

Limits: points are non-zero whole numbers from -1000 to 1000, caps up to 100000, decay from 1 to 3650 days, up to 50 criteria per model and 50 models.

The first model of a record type becomes its **primary** model, shown in the read-only **Score** field of lists and detail pages. You can make another model primary, deactivate it, recalculate it or delete it (deleting removes every score it calculated). The **Score** tab of a contact or company explains what made up the score. Scores are recalculated on every relevant event, when a model is saved, and once a day to apply decay.

Automations can react when a score crosses into another band; see [Automations](/_aphel/docs/using/automations/).

## Lead distribution

Distribution rules pick the owner of new records automatically. Administrators with `assignment.manage` create them in **Settings > Lead distribution**. A rule is used by an automation's **Assign by rule** action and by other places that create records, such as incoming webhooks, imports, lead forms and bookings. The history records which of these sources triggered each assignment.

A rule has a record type, a list of members, a strategy and, optionally, routing.

| Strategy | Who gets the next record |
| --- | --- |
| Round robin | Whoever waited longest. |
| Weighted round robin | Members receive in proportion to their weight, from 1 to 100. |
| Least loaded | Whoever has the fewest active records of this type. |

- **Away**: mark a member as away, optionally between two dates. Without dates, they stay away until you uncheck it. Suspended users and users outside the organization are skipped.
- **Routing by field**: send records to a team or specific users according to the value of a field, such as region or source. The first matching route wins, and values are compared ignoring upper and lower case. When no route matches, the rule either distributes among its members or leaves the record without an owner.
- **Simulate** shows who would receive the next records without saving anything. **History** lists who received each record.

A rule can have up to 200 members and 50 routes. Deleting a rule erases its rotation state and history, and automations that still use it stop assigning.

## Lead Pro (optional plugin)

Lead Pro adds a **temperature** to contacts: interactions warm a lead up, and idle leads cool down. It is a separate plugin, kept outside the default build and not part of any blueprint, so it exists only in installations that include it. When present:

- A **Hot leads** page lists contacts at or above the hot temperature, hottest first. It needs `lead_pro.read`, granted to Admin, Manager and Member by default.
- Each contact has a **Temperature** tab with the latest touches and a **Heat up** button that registers a manual touch.
- Activities, notes, emails sent or received and booked meetings count as touches. Each touch adds points, up to 100.
- Qualifying a lead requires a temperature at or above the hot threshold; otherwise the lead is refused with a "not hot enough" error.
- When a lead becomes hot, its owner is notified, and the conversion records the temperature in the deal's source.

Defaults, all editable in the plugin settings: hot threshold 70, initial temperature 20, 10 points per touch, 5 points lost per day after 7 days without interaction (cooling runs daily at 03:00).

## Gotchas

- A contact must be in the **Lead** stage to appear in the inbox, and a manually created contact has no stage until you set one.
- Qualified leads are not in the inbox; find them with the stage filter.
- Disqualifying works on both **Lead** and **Qualified** contacts, but the inbox only lists leads.
- If **Disqualify** says there is no active reason, an administrator must add one in **Settings > Lifecycle**.
- Scores and temperature are different: scores come from your scoring models, temperature only exists with Lead Pro.
