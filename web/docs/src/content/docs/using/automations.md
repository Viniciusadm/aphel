---
title: Automations and webhooks
description: Build no-code workflows that react to events and time, and connect Aphel to other systems with outgoing and incoming webhooks.
sidebar:
  order: 10
---

Automations run actions on records without anyone clicking: create a follow-up task when a deal reaches a stage, notify a manager when a deal stalls, call an external URL when a deal is won. Webhooks are the other half of integration: outgoing webhooks send Aphel events to other systems, and incoming webhooks let other systems create or update records in Aphel.

:::note
Automations, approvals and webhooks are modules. Automations are suggested by the Agency, Freelancer, Other business, Real Estate, SaaS and B2B Sales blueprints; webhooks by the SaaS blueprint; approvals by the Consultancy and B2B Sales blueprints. An administrator can turn any of them on or off in **Settings > Modules**. See [Blueprints](/docs/admin/blueprints/) and [Modules](/docs/admin/modules/).
:::

## Who can use what

| Area | Where | Permission |
| --- | --- | --- |
| Automations | **Automations** in the main menu | `automations.manage` |
| Outgoing webhooks | **Settings > Integrations > Outgoing webhooks** | `settings.manage` |
| Incoming webhooks | **Settings > Integrations > Incoming webhooks** | `settings.manage` |
| Approval policies | **Settings > Sales > Approvals** | `approvals.manage` |
| Deciding on approvals | **Approvals** in the main menu | You must be an approver of the current step |

By default only the Administrator role has `automations.manage`, `approvals.manage` and `settings.manage`. You can grant them to other roles in [Roles and permissions](/docs/admin/roles-and-permissions/). Without `automations.manage` the Automations screen shows "No access to automations".

## How an automation works

An automation has four parts:

- **Record type.** The kind of record it works on: Contact, Company, Deal, Activity, or a custom object. It cannot be changed after creation.
- **Trigger.** What starts a run.
- **Steps.** A workflow of conditions, branches, waits and actions, built visually.
- **Versions.** You edit a draft and publish it. Only the latest published version runs.

Every automation is in one of three states:

| State | Meaning |
| --- | --- |
| Draft | Never published. It does not run. |
| Active | The published version runs when its trigger fires. |
| Paused | Published before, but switched off. |

Saving a draft never changes what is running. A change goes live only when you click **Publish**, which creates the next version and makes the automation Active.

### Whose permissions an automation uses

Published automations act with the permissions of **the user who published that version**, reloaded at every run. Two consequences:

- When you publish, Aphel checks that you hold every permission the actions need (for example `tasks.create` for Create task, `settings.manage` for Call webhook). If not, the editor reports an issue and you cannot publish.
- If the publisher later loses a permission, the affected step fails at run time. If the publisher is no longer available or cannot see the record, the run is skipped.

## Create an automation

1. Open **Automations** and click **New automation**.
2. Pick a ready-made example to adapt (for example "Follow up on sent proposal", "Deal stalled for 7 days", "Reach out again in 2 days", "First touch with new contact", "Celebrate won deal"), or choose **Start from scratch**.
3. For a blank automation, enter a name, optionally a description, the **Record type** and the **Trigger**, then click **Create draft**.
4. Build the workflow in the editor (next section).
5. Click **Save draft**, test it, then click **Publish**.

The list shows each automation's status, version, when it was last triggered, runs in the last 30 days, error rate and how many runs are waiting. You can search by name or key and filter by status and record type.

Limits: up to 200 automations per organization, 140 characters for the name, 1000 for the description.

## Triggers

| Trigger | Fires when |
| --- | --- |
| Record created | A record of the type is created. |
| Record updated | A record changes. Optionally list **Watched fields**; leave it empty to fire on any change. |
| Record deleted | A record is deleted. The record itself is gone, so conditions about it evaluate to false. |
| Deal stage changed | A deal moves to another stage. You can narrow it by the stage it left and the stage it entered. |
| Deal won / Deal lost | A deal is marked won or lost. |
| Activity completed | An activity is completed. On other record types it runs for the records the activity is linked to. |
| Note created | A note is added to a record. |
| Manual | Only when someone runs it on a record. |
| Incoming webhook | A specific [incoming webhook](#incoming-webhooks) creates or updates the record. |
| Record date | A period before or after a date field is reached, for example 3 days before the expected close date. You choose the field, the offset and, for date fields, the time of day. It runs once per record and field value. |
| Stalled record | A record goes a number of days (1 to 3650) with **No updates**, **No activity** (deals) or **Same stage** (deals only). A new stop counts only after the record changes again. |
| Deal rotting in stage | An open deal goes past the day limit of its stage. Once per stage entry. |
| Scheduled | A cron schedule fires, once per record matching an optional filter or segment. |

Scheduled triggers use six cron fields (second, minute, hour, day, month, day of week) in the organization time zone. Occurrences must be at least 15 minutes apart. The editor offers templates such as "Every day at 9 AM" and "Weekdays at 9 AM".

Other modules add triggers when they are enabled, for example a lead form being submitted, a meeting being booked, rescheduled or canceled, a proposal being generated, approval requested, approved, rejected or cancelled, and e-signature events. These appear in the trigger list for the record types they support. Their names come from the module that provides them and may not be translated.

## Build the workflow

Click a block to configure it, and use **+** to add steps. Steps are grouped as **Logic**, **Time** and **Actions**.

| Step | What it does |
| --- | --- |
| Condition | Continues only if the conditions are met. Otherwise the run ends there, without an error. |
| Branch | Follows the first path whose conditions match, or **Otherwise**. Up to 10 paths. |
| Action | Does something (see below). |
| Wait | Pauses for a period (minutes, hours, days or weeks) or until a date on the record, with an optional offset such as 1 day before. **Recheck before continuing** makes the run proceed only if some conditions are still true, for example the deal is still open. |
| Wait until condition | Pauses until conditions match, rechecking about every 10 minutes. You set a maximum wait; when it runs out the run follows **Timed out** or ends. |

An automation can have at most 50 steps. Every path must end; loops are not allowed, and every step must be reachable from the start. Publishing requires at least one action.

### Conditions

Conditions can test fields of the record and also facts about the trigger: which fields changed, who made the change (User, System, API token or Automation) and the previous value of a field. You can also test related records, for example "there is no open follow-up task".

### Variables in text

Text fields such as task titles, notification messages and notes accept variables. Click **Insert variable** to pick them. They are grouped as Record (title, owner, related names, any field), Trigger (event, changed fields, previous values), Dates (now, today, tomorrow, in a number of days), Automation and Run.

### Wait settings

Under the workflow options you decide what happens to runs sitting at a wait step:

| Option | Default |
| --- | --- |
| Cancel waiting runs when the automation is paused | On |
| Cancel waiting runs when a new version is published | Off |

## Actions

| Action | What it does |
| --- | --- |
| Create task | Creates a task linked to the record. The assignee defaults to the record owner. Set a due time relative to the run ("Due in") or a date variable. |
| Update fields | Sets fields on the record. An empty value clears the field. |
| Change owner | Sets the owner to a fixed user or to a user stored in a field. |
| Add tag / Remove tag | Adds or removes a tag. |
| Move stage | Moves a deal to a stage. Stage requirements and lost reasons still apply. |
| Create note | Adds a note to the record. |
| Send notification | Shows an in-app notification to the record owner, specific users or users from a field. |
| Send email | Sends an email to the recipients, respecting their notification preferences. |
| Call webhook | Sends an HTTP request to an external URL. Needs `settings.manage` to publish. |
| Create related record | Creates a record of another type and can link it to the current one. |
| Assign by rule | Assigns the owner using a lead distribution rule. |
| Enroll in sequence | Enrolls a contact in a sequence. Emails go out from the contact owner's mailbox or from the mailbox of whoever published the automation. |

Other modules contribute more actions when enabled, such as requesting an approval, generating a proposal, sending an email from the owner's mailbox and enriching a company. Actions are limited to the record types they support, and each one checks the permissions it needs when you publish.

### Call webhook

The URL is fixed (it cannot contain variables). You can choose the method (POST by default; GET and DELETE send no body), add up to 20 extra headers, and set the body, which defaults to the automation, run and record ids plus the record. An optional **Signing secret** (at least 16 characters) is stored encrypted, never shown again, and used to sign the call in the `X-Aphel-Signature` header. The same protection against internal addresses applies as for outgoing webhooks.

Tick **Safe to retry on failure** only if receiving the same call twice causes no duplicate effect. Without it, a failed call is not retried.

## Test before you publish

Open the **Test** tab, choose a record and click **Run test**. The workflow runs in simulation: conditions and branches are evaluated against real data, and each action shows what it would do, with sensitive values masked. Nothing is saved and no message is sent. By default the draft is tested; clear **Test the draft** to test the published version. Unsaved changes are saved before the test.

For automations with the Manual trigger, the same tab also offers **Run for real**, which queues an actual run on the record. It works only on an active automation and a record you can read.

## Runs and history

The **Runs** tab lists runs with status, version, trigger and record; you can filter by status and by record. Open a run to see its steps with input, output, timing and the condition result.

| Run status | Meaning |
| --- | --- |
| Running | In progress. |
| Waiting | Paused at a wait step; the run shows when it resumes. |
| Succeeded | All steps finished. |
| Failed | A step failed. The step shows the error. |
| Skipped | Not run. The reason is shown (see below). |
| Cancelled | Cancelled while waiting, for example because the automation was paused. |

A run is skipped, not failed, in these cases:

| Reason | Meaning |
| --- | --- |
| Automation chain too long | Automations triggering automations went more than 5 levels deep. |
| Cycle between automations | An automation would trigger itself through a chain. |
| Hourly run limit reached | The same automation ran more than 20 times in an hour on the same record. |
| Record not found / outside the publisher's access | The record is gone or the publisher cannot see it. |
| The publisher is no longer available | The user who published it is not an active member. |
| Automation inactive | It was paused or is not active. |

When loop protection or the run limit stops an automation, its creator receives an "Automation stopped" notification, at most once an hour per reason. You can change this notification in your notification preferences.

The **Versions** tab lists every published version with who published it and when. **View workflow** shows a version, and **Restore to draft** copies it into the draft; nothing changes in production until you save and publish.

## Pause, activate and delete

Use **Pause** and **Activate** in the editor header or the list. An automation that was never published cannot be paused or activated. **Delete automation** under **More actions** removes its versions, run history and stored secrets, and cannot be undone.

## Outgoing webhooks

Go to **Settings > Integrations > Outgoing webhooks**. Each endpoint receives a JSON POST for the events you choose, signed with the endpoint's secret.

### Create an endpoint

1. Click **New endpoint**.
2. Enter the **URL**. It must be public: internal addresses (localhost, private networks) are blocked, and https may be required (see the delivery settings below). Maximum 2048 characters.
3. Optionally add a description (up to 255 characters).
4. Choose the **Events**: pick specific events, or **All events, including ones added in the future**. Events cover contacts, companies, deals, activities, notes and the events of other modules.
5. Leave **Active** on and click **Create endpoint**.
6. Copy the signing secret from the dialog. It is shown only once. Receivers use it to verify the `X-Aphel-Signature` header.

An organization can have up to 20 endpoints. The body format and signature check are described in [Webhooks and events](/docs/reference/webhooks-and-events/).

### Manage endpoints

From the actions menu of an endpoint you can **Edit**, **Send test event**, **View deliveries**, **Generate new secret**, **Disable** or **Re-enable**, and **Delete**. Generating a new secret takes effect immediately, including for deliveries waiting to be retried, so update the receiving system right away. Deleting an endpoint also erases its delivery history.

### Deliveries and retries

**View deliveries** shows the last 30 days (configurable): status (Pending, Success, Failed), the event, attempts and the response. Open a delivery for the request body and response, and use **Redeliver** to send it again.

Failed event deliveries are retried up to 7 attempts in total, spaced from 1 minute to 12 hours. Test events and redeliveries are tried once.

When an endpoint fails too many times in a row, Aphel disables it ("Disabled due to failures") and notifies people who manage settings. Fix the destination, then choose **Re-enable**.

### Delivery settings

A **Delivery** panel on the same page applies to new deliveries without a restart:

| Setting | Default | Range in the screen |
| --- | --- | --- |
| Maximum time per delivery (s) | 10 | 1 to 60 |
| Consecutive failures before disabling | 15 | 1 to 1000 |
| Days of delivery history | 30 | 1 to 365 |
| Require https in endpoint URLs | Off | On or off |

## Incoming webhooks

Go to **Settings > Integrations > Incoming webhooks**. An incoming webhook is a secret URL: when another system sends data to it, Aphel runs the actions you configured to create or update records.

### Create an incoming webhook

1. Click **New webhook**. You can start from **Template: website form**.
2. Enter a name (up to 100 characters) and optionally a description.
3. Add **Actions**, in order. Each action is either **Create record** or **Create or update record** (matches a unique field and updates or creates). Choose the entity, and map each incoming value to a field. Use `$.payload.path` for incoming data and `$.actions[0].id` for the result of an earlier action. Up to 10 actions.
4. Optionally tick **Require HMAC signature** to generate a signing secret, and set the **Signature time window** (30 to 86400 seconds).
5. Keep **Active** on and click **Create webhook**.
6. Copy the URL (and secret) from the dialog. They are shown only once, and anyone holding the URL can send data to your CRM.

For contacts, you can decide what happens when one already exists: update the one with the same email, the same phone or mobile, the same value in a unique custom field, or always create a new one. Mark a custom field as unique in the field settings to use it.

Actions run as the user who created the webhook, with that user's permissions at the time of each request. If an action fails, the earlier ones are kept and the partial result is logged. A webhook with no actions only logs requests.

An organization can have up to 50 incoming webhooks. The receiver accepts JSON and form-encoded bodies up to 256 KB. A signed request needs the `X-Aphel-Signature` header, and repeating the same delivery within 24 hours is refused. Details are in [Webhooks and events](/docs/reference/webhooks-and-events/).

### Manage and inspect

The menu of each webhook offers **Example request (cURL)**, **Generate new URL** (the old URL stops working immediately), **Require signature** / **Generate new secret** / **Remove signature**, and **Delete** (which erases the log).

**Log** shows requests from the last 30 days with status (Success, Partial, Failed or Rejected), the result of each action, headers (sensitive ones are not stored) and the body, truncated at 8 KB.

To react to what a webhook created, build an automation with the **Incoming webhook** trigger and select that webhook. The received body (up to 16 KB) is available to the automation as trigger data.

## Approvals

The approvals module lets you require a sign-off before something happens. Policies are managed in **Settings > Sales > Approvals** with `approvals.manage`.

Each policy has a name, a unique key (lowercase letters, digits and `_`, fixed after creation), a record type and a **Moment**:

| Moment | When it applies |
| --- | --- |
| Stage change | Moving a record to a chosen target stage (only for types with stages, such as deals). |
| Quote sending | Sending a quote to the customer. See [Quotes and proposals](/docs/using/quotes-and-proposals/). |
| On demand | Only when someone, or an automation, requests approval. |

A policy also has a **Condition** (always, when the record matches a filter, or when a formula is true), optional **Locked fields** that cannot change while a request is pending (none selected locks the whole record), an optional **Due (hours)** from 1 to 2160, and up to 10 steps. Each step names an approver (a user, a role, or the owner's team manager) and a rule: **Anyone approves** or **Everyone must approve**. Without a team manager, administrators with `approvals.manage` approve. The requester never approves their own request. The order of policies sets their priority.

### Requesting and deciding

When a stage change or a quote send needs approval, the action is refused with "This action requires approval". Click **Request approval** (add a comment if useful), wait for the decision, then repeat the action. An approval is used once. You can also request approval from the **Approvals** tab of a deal.

Approvers see requests under **Approvals > For me**, and requesters under **Requested by me**; anyone with `approvals.manage` also sees **All**. Approving is optional commentary; **rejecting requires a reason**. The requester or an approvals administrator can **Cancel request**. Statuses are Pending, Approved, Rejected and Cancelled.

Policies deleted later do not affect requests already open: they keep their own copy of the steps.

### Approvals and automations

- The **Request approval** action asks for approval on the record under a given on-demand policy. Nothing happens if the policy condition does not match or a request is already open.
- Triggers for approval requested, approved, rejected and cancelled can start automations, optionally for one policy key.

## Gotchas

- **Publishing matters.** Editing the draft of an active automation does not change what runs until you publish.
- **Publisher permissions.** If the person who published leaves or loses access, their automations start failing or being skipped. Re-publish them from an administrator account.
- **Rapid loops.** An automation that updates the same record it listens to can retrigger itself. The chain depth, cycle and hourly limits stop it, but design triggers with Watched fields and conditions instead.
- **Waiting runs.** Pausing cancels waiting runs by default, so pausing and re-activating does not resume them.
- **Webhook URLs and secrets are shown once.** Store them when created; to recover, generate a new URL or secret and update the other system.
