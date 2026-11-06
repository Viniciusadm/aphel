---
title: Forms and booking
description: Publish lead capture forms and public booking pages so visitors can reach you without logging in.
sidebar:
  order: 11
---

Two modules give outsiders a way into Aphel without an account: **Lead forms** turn website submissions into contacts (and optionally deals), and **Booking** lets clients pick a free time in your calendar.

:::note
Both are optional modules. Lead forms are suggested by the Real Estate blueprint. Booking is suggested by the Agency, Consultancy, Freelancer and Real Estate blueprints. An administrator can turn them on or off in **Settings > Modules**. See [Modules](/docs/admin/modules/) and [Blueprints](/docs/admin/blueprints/).
:::

## Permissions

| Area | Menu item | Permission | Default roles |
| --- | --- | --- | --- |
| Lead forms | **Forms** | `forms.manage` | Administrator |
| Booking | **Booking** | `booking.manage` | Administrator, Manager, Member |

Grant `forms.manage` to other roles in [Roles and permissions](/docs/admin/roles-and-permissions/). `booking.manage` is scoped: Members manage their own links and availability, Managers also those of their team, and team links need at least team scope.

## Lead forms

Open **Forms**. Each form has a public address you can embed in your website or share. Every submission creates a contact (or reuses an existing one) and, if configured, opens a deal.

### Create a form

1. Click **New form**.
2. **General.** Enter a name (up to 120 characters). Under **Create records as**, choose whose permissions are used to create contacts and deals; the default is whoever created the form. That user must be an active member who can create contacts (and deals, if the form creates deals). If they later lose access, submissions fail and appear in the history as failed.
3. **Fields.** Add up to 30 fields. For each: label, identifier (lowercase letters, digits and `_`, up to 40 characters), type, placeholder and whether it is required. Types are Text, Email, Phone, Long text, Options list and Checkbox. For options lists, write one option per line, using `value | label` when the stored value differs from the shown text.
4. **Save on the contact as.** Map each field to a contact field, system or custom, or choose **Do not save (history only)**. A field mapped to the contact name must be required, and one field must map to the first name or to **Full name** (which splits first and last name). Two fields cannot map to the same contact field, and checkboxes map only to checkbox fields.
5. **Contact.** Tick **Reuse the contact when the email already exists** to avoid duplicates. Set a **Source** and an optional **Tag** to apply. New contacts get the lifecycle stage "lead" when that field exists.
6. **Deal (optional).** Tick **Create a deal linked to the contact**, then choose the pipeline, the stage, a title and an optional amount. In the title, use the field identifier in double braces, such as `{{name}}`, to insert answers, and `{{form}}` for the form name.
7. **Owner.** Choose a fixed owner, and optionally a **Distribution rule** (lead distribution module). With a rule, each submission goes to the rule's next available person; if nobody is available, the fixed owner or the acting user is used. Existing contacts keep their current owner.
8. **After submission.** Set the button text, the success message and an optional redirect address (http or https).
9. **Security.** Optionally list **Allowed websites** (up to 20, one per line, as `https://www.example.com`). When filled, only those sites can show the form in an iframe and submit through the script.
10. Leave **Form active** on and click **Create form**.

### Publish it

Click **Embed** on the form. You get three options:

| Option | Use it for |
| --- | --- |
| Iframe | A ready page isolated from the rest of the site; works without JavaScript. |
| Script | Renders the form inside your page and inherits the site font. |
| Direct link | Sharing on social networks, email or a QR code. |

The codes work only while the form is active.

### Review submissions

The form card shows how many submissions were accepted, flagged as spam and failed, plus the date of the last one. **Submissions** lists the history and can filter by status:

| Status | Meaning |
| --- | --- |
| Accepted | A contact was created or reused. If the tag or the deal could not be applied, the submission is still accepted and shows the warning. |
| Spam | A hidden honeypot field was filled in. Nothing was created. |
| Failed | Aphel could not create the records, for example because the acting user lost access. |

Each entry shows whether the contact was created or existing, the linked deal, the origin and the IP address.

### Other actions

From **More actions**: **Generate a new public address** (the old codes and links stop working at once, so update your site) and **Delete** (the form stops working and its submission history is erased; contacts and deals already created stay).

### Spam protection and limits

A honeypot field and rate limits protect every form: up to 5 submissions per minute from one IP address and 300 per hour per form; a submission body is limited to 64 KB. Visitors who hit a limit see an error and can try again later.

### Automate on submission

Lead forms provide an automation trigger fired when a form is submitted, for the Contact record type, so you can create a task or notify the owner. See [Automations and webhooks](/docs/using/automations/). Submissions also publish an event for [outgoing webhooks](/docs/reference/webhooks-and-events/).

## Booking

Open **Booking**. It has three tabs: **Meeting types**, **My availability** and **Bookings**. Booking depends on the tasks module because each booking becomes a meeting activity.

### Set your availability

1. Open **My availability**. Until you save, it shows defaults (Monday to Friday, 09:00 to 18:00) and "Not configured". **Save to start receiving bookings.**
2. Choose your **Time zone**.
3. Set **Weekly hours**: add intervals per weekday (up to 28 per week, no overlaps, end after start). Days without intervals are unavailable.
4. Set the limits:

| Setting | Default | Range |
| --- | --- | --- |
| Minimum notice (minutes) | 240 | 0 to 43200 |
| Maximum meetings per day | No limit | 1 to 50 |
| Booking window (days) | 60 | 1 to 365 |
| Buffer before / after (minutes) | 0 | 0 to 240 |

Managers can open the availability of people they have access to with the **Availability of** selector.

Free times are computed from your weekly hours minus your existing meetings and bookings. If Google or Microsoft calendars are connected, busy times from them are also excluded. If a connected external calendar cannot be read, your times are hidden rather than risking a double booking.

### Create a meeting type

1. In **Meeting types**, click **New meeting type**.
2. Enter a name (up to 140 characters) and optionally a description.
3. Set the **Duration** (5 to 480 minutes) and the **Interval between times** (5 to 240 minutes), which controls how often start times are offered.
4. Choose a **Location**: No location, Video call, Address or Phone, and enter the link or details. The location is shown to the guest only after booking.
5. Set the **Link address** (lowercase letters, numbers and hyphens, at least 3 characters). Leave it empty to generate it from the name. Changing it later breaks the old link. `manage` and `public` are reserved.
6. Add up to 10 **Questions** (short or long text, optionally required). Each can fill a contact field when the guest becomes a new contact.
7. Keep **Spam protection** on (honeypot and attempt limit) and **Active** on, then save.

Use **Copy link** or **Open** to share the public page, available at `/book/<link address>`. Deleting a meeting type stops its link; existing bookings are kept.

### Team links

A **Team link** takes a **Distribution rule**. The visitor sees a time when any member of the rule is free, and the booking is assigned to a team member according to the rule. Members who are away are skipped. You need `booking.manage` with at least team scope to create team links. A personal link uses only the owner's availability.

### What visitors do

A visitor opens the link, picks a day and time (shown in their time zone, which they can change), enters name and email, answers your questions and confirms. The booking is final unless the time was just taken.

Aphel then:

- finds a contact with that email or creates one, owned by the host, filling mapped fields from the answers;
- creates a meeting activity for the host, linked to the contact;
- emails the guest the details. The guest gets a private link to **Reschedule** or **Cancel** the booking.

Public links are rate limited: 10 bookings per hour per address and 200 per hour per link, and 30 changes per hour per address.

### Manage bookings

The **Bookings** tab lists bookings with status **Confirmed** or **Canceled**; filter by meeting type, status, or **Upcoming only**. Click **Cancel** to cancel a booking, with an optional reason; the guest is notified. Bookings that can no longer be changed show "This booking can no longer be changed."

### Automate on bookings

Booking provides automation triggers when a meeting is booked, rescheduled or canceled, for the Contact record type. See [Automations and webhooks](/docs/using/automations/). The same events are available to outgoing webhooks.

## Gotchas

- **Forms keep the acting user.** Removing that person or reducing their permissions makes submissions fail. Change **Create records as** first.
- **A new public address replaces the old one.** Embedded codes on your site must be updated.
- **Booking needs a saved availability.** A host who never saved their availability still gets the defaults, but check the time zone before sharing links.
- **Changing a booking link address breaks old links** that were already shared.
