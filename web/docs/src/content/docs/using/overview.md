---
title: Overview
description: How the Aphel menu is organized, why different workspaces show different screens, and how to use Aphel on a phone.
sidebar:
  order: 1
---

This page explains where things are in Aphel, why your menu may look different from a colleague's, and how to make the workspace fit the way you work. The rest of the "Using Aphel" section describes each area in detail.

## What you see depends on four things

Aphel is built from modules. A module is a feature such as Contacts, Deals, Quotes or Automations. Whether an entry shows up in your menu depends on:

1. **Which modules are enabled** in your workspace. An administrator chooses them when the workspace is created and can change them later under **Integrations > Modules**.
2. **The blueprint** the workspace started from. The blueprint is the business type chosen during setup (for example B2B sales or Freelancer). It decides which modules are suggested and which starting configuration is applied. See [Blueprints and modules](#blueprints-and-modules).
3. **Your role.** Every menu entry is tied to a permission such as `deals.read`. If your role does not include it, the entry is hidden. See [Roles and permissions](/_aphel/docs/admin/roles-and-permissions/).
4. **Your personal preferences.** You can reorder entries, hide the ones you do not use and choose your home screen. See [Make the menu yours](#make-the-menu-yours).

If a screen described in this documentation is missing for you, one of these is the reason. Ask your administrator which one before assuming something is broken.

## The menu

The sidebar is split into five sections. A section only appears when it contains at least one entry you can open.

| Section | What it holds |
| --- | --- |
| My work | Your daily screens: Home, Tasks, Inbox, Records I follow, Calendar and the messaging Inbox. |
| CRM | The records you sell to and with: Contacts, Leads, Companies and Deals, followed by any custom objects. |
| Operations | Everything that supports the work: Products, Quotes, Segments, Sequences, Booking, Calls, Approvals, Automations, Forms, Proposals and the AI assistant. |
| Insights | Reports, Goals, Forecast and the calls report. |
| Administration | Settings, System and the sign-in and calendar connectors (Single sign-on, Google, Microsoft). |

Entries that a module does not assign to a section land in **Operations**.

### Entries and the permission that opens them

The table lists the main entries in the order they appear inside their section. An entry that needs several permissions is shown when you have at least one of them. A dash means the entry has no permission requirement of its own.

| Section | Entry | Needs |
| --- | --- | --- |
| My work | Home | none |
| My work | Tasks | `tasks.read` |
| My work | Inbox (tasks) | `tasks.read` |
| My work | Records I follow | none |
| My work | Calendar | `calendar.read` |
| My work | Inbox (messaging) | `messaging.view` |
| My work | Messaging channels | `messaging.manage_channels` |
| CRM | Contacts | `people.read` |
| CRM | Leads | `people.read` |
| CRM | Companies | `organizations.read` |
| CRM | Deals | `deals.read` |
| CRM | Custom objects | `custom.<key>.read` of each object |
| Operations | Products | `catalog_items.read` |
| Operations | Quotes | `quotes.read` |
| Operations | Segments | `people.read`, `organizations.read` or `deals.read` |
| Operations | Sequences | `sequences.read` or `sequences.manage` |
| Operations | Booking | `booking.manage` |
| Operations | Calls | `tasks.read` |
| Operations | Approvals | none |
| Operations | Automations | `automations.manage` |
| Operations | Forms | `forms.manage` |
| Operations | Proposals | `proposals.manage` |
| Operations | AI assistant | `ai.use` or `ai.view_usage` |
| Insights | Reports | none |
| Insights | Goals | `deals.read`, `tasks.read` or `people.read` |
| Insights | Forecast | `deals.read` |
| Administration | Single sign-on | `settings.manage` |
| Administration | Google | `google.calendar_sync` or `settings.manage` |
| Administration | Microsoft | `microsoft.calendar_sync` or `settings.manage` |
| Administration | System | `system.manage` |
| Administration | Settings | access to at least one settings area |

:::note
The Settings entry is always the last item of the menu and cannot be hidden or moved. It appears as soon as you can open at least one settings screen.
:::

### Custom objects

Custom objects are record types that your administrator created, such as Contracts or Properties (see [Custom fields](/_aphel/docs/using/custom-fields/#custom-objects)). Each active object appears in the **CRM** section, after the built-in records, under its plural name and with its own icon and color. You see an object only when your role has `custom.<key>.read` for it. Inactive objects disappear from the menu while keeping their data.

### Settings areas

Settings is split into four areas. Each area only lists the screens you are allowed to open.

| Area | Purpose | Typical screens |
| --- | --- | --- |
| My account | Your own preferences and sign-in security. Available to everyone. | Appearance, Preferences, Navigation and language, Account security, Security events, API tokens |
| Settings | How the organization works: access, record structure and sales. | System email, Organization, General, File storage, Users, Teams, Roles, Fields, Layouts, Validation rules, Custom objects, plus the Sales group |
| Integrations | Connections to other services and the module list. | Modules, webhooks and the connectors you enabled |
| Data | Import, export and data care. | Audit log, Trash, Privacy, Full export and similar |

Most organization screens need `settings.manage`. Users need `users.manage`, the Audit log needs `audit.read`, and System needs `system.manage`. Administrators are covered in [Users and teams](/_aphel/docs/admin/users-and-teams/) and [Roles and permissions](/_aphel/docs/admin/roles-and-permissions/).

## Make the menu yours

Open **My account > Navigation and language**. The changes apply only to you, in every organization you belong to.

**Language.** Choose "Follow the organization" or a specific language. The choice applies to the interface and to the emails you receive, and dates and numbers follow it.

**Home screen.** Choose where you land when you sign in. The default is Home (the dashboard). Any entry you can open can be your home.

**Menu items.**

1. In **Menu items**, use **Move up** and **Move down** next to an entry to reorder it.
2. Clear the checkbox next to an entry to hide it.
3. Select **Restore default** to undo your ordering and hiding.
4. Select **Save navigation**.

New entries that appear later (for example after a module is enabled) are added at the end of your order. **Reset only my home and navigation** clears just this part of your preferences.

:::tip
Press Ctrl+K (Cmd+K on a Mac) to open the command palette. It searches records and jumps to any screen, so you rarely need the sidebar for entries you hide. Single-key shortcuts such as "g c" for contacts, "c" to create and "?" for help can be changed or turned off under **My account > Keyboard shortcuts**.
:::

## Where you land after signing in

Aphel picks your starting screen in this order, using the first one that applies:

1. The home screen you chose in **Navigation and language**.
2. A home defined for your role by a plugin, if any.
3. The home defined by the blueprint (only when it is not the plain home page).
4. On a phone-sized screen (narrower than 768 pixels), the **Today** screen.
5. Otherwise Home.

The redirect happens once per browser session, when you open the root address.

## Blueprints and modules

A blueprint is a small file that describes a business type. During setup, the **Business** step asks what kind of business you run, what you sell and how many people will use Aphel. Your answer selects the blueprint, and the **Features** step then turns on what usually helps that kind of business. You can adjust every choice, or start with a blank system, and you can change it later.

Blueprints are layered. Every business type except the base one extends **CRM**, so it receives everything CRM provides and adds its own suggestions. A blueprint can declare:

- **Required modules.** Always installed, together with the modules they depend on.
- **Recommended modules.** Pre-selected in the Features step. You can untick them.
- **Configuration packages.** Starting configuration applied to a new workspace: custom fields, pipelines and stages, lost reasons, activity types, tags, form layouts, saved views and the default dashboard. Packages never carry business data.
- **Navigation.** A home screen and, optionally, its own sections that regroup modules in the menu.
- **Roles.** Extra roles the blueprint creates.

The built-in blueprints do not define custom menu sections or extra roles. They only differ in recommended modules and configuration packages.

### What the base CRM blueprint includes

Required: People (Contacts), Organizations (Companies), Deals, Notes, Tags, Integrations, Exports and Full export.

Recommended: Timeline, Dashboard, Reminders, Favorites, Followers, Imports, Bulk actions, Duplicates, Trash, Privacy, Marketplace, Notifications, Objects (custom objects), Search, Saved views, Record layouts, Formulas, Validation rules, Configuration packages, Calendar, Tasks, Workflow (pipelines), Money and Documents. The WebAssembly runtime (runtime plugins) is part of every installation.

### What each business type adds

Each blueprint below also contains everything from CRM.

| Blueprint | Adds these modules | Configuration package |
| --- | --- | --- |
| B2B Sales | Leads, Assignment, Scoring, Goals, Forecast, Reports, Calls, Email, Sequences, Segments, Automations, Approvals, Google, Microsoft, Single sign-on, AI assistant, CNPJ enrichment | sales |
| Freelancer | Booking, Products, Quotes, Email, Automations, Google, AI assistant | freelancer |
| SaaS | Leads, Scoring, Line items, Deal revenue, Renewals, Goals, Forecast, Segments, Automations, Webhooks, Extra widgets, Google, Microsoft, Single sign-on, AI assistant | saas |
| Consultancy | Booking, Products, Line items, Deal revenue, Quotes, Proposals, Approvals, Email, Reports, Google, Microsoft, AI assistant, Electronic signature | none of its own (CRM only) |
| Real Estate | Leads, Assignment, Lead forms, Calls, Messaging channels, Booking, Email, Automations, Google, AI assistant, CNPJ enrichment, Electronic signature | real_estate |
| Other business | Leads, Products, Quotes, Email, Automations, Reports | none of its own (CRM only) |
| Agency | Leads, Products, Line items, Deal revenue, Price lists, Quotes, Proposals, Email, Sequences, Segments, Automations, Reports, Booking, Google, Microsoft, AI assistant, CNPJ enrichment, Electronic signature | agency |

This is why a freelancer sees Quotes and Booking from day one while a B2B sales team sees Sequences, Goals and Forecast, and why Leads is missing from the Freelancer and Consultancy menus until someone enables it.

Modules that are not in your blueprint are not gone. Administrators with `settings.manage` can enable any available module under **Integrations > Modules**.

### Enabling and disabling modules

Disabling a module stops its screens, menus, widgets and automatic routines for the whole organization, but keeps its data. Enabling it again restores everything. A module that other enabled modules depend on cannot be disabled until those are disabled first, and modules the build marks as required cannot be disabled at all. Removing a module's data is a separate, permanent action that requires disabling and exporting first. Details are in [Modules](/_aphel/docs/admin/modules/) and [Blueprints](/_aphel/docs/admin/blueprints/).

## Using Aphel on a phone

Aphel is a web app that adapts to small screens and can be installed on the home screen as a PWA (progressive web app).

**Layout on small screens.** Screens narrower than 768 pixels get a bottom bar with **Today**, a shortcut to **Deals** (when the Deals module is enabled and you can read deals), **Search** and **Menu**. **Menu** opens the full sidebar. Lists switch to a compact toolbar, where filters and sorting are applied together when you confirm.

**Quick entry.** The create menu opens a quick-entry sheet for logging things in a few taps. The options depend on the modules you have and your permissions: **Contact** (`people.create`), **Note** (`notes.create`), **Activity** and **Check-in** (`tasks.create`) and **Call** (`tasks.create`). When you start from a record page, the new item is linked to that record; otherwise you can search people, organizations or deals to link it. Notes and activities support dictation when your browser offers speech recognition and you allow the microphone.

**Install.** When your browser supports it, **Install Aphel** appears so you can add the app to your home screen. On iPhone and iPad, tap Share, then Add to Home Screen, and open Aphel from the icon.

**Updates.** When a new version is available, a banner offers **Update**. Anything unsaved on the screen can be lost when you update, so finish what you are typing first or choose **Later**.

**Push notifications.** From your account settings you can turn push notifications on for the current device (look for **Push notifications on this device**). They deliver the notices checked in the Push column of your notification settings even when Aphel is closed. The setting is per browser and turns off when you sign out. If notifications are blocked for the site, allow them in the browser settings first. On iPhone and iPad they work only with the installed app.

**Offline.** If the connection drops, an **Offline** indicator appears. Notes, activities, contacts and calls you create are saved on the device and sent automatically when you reconnect. The indicator shows how many items are pending or failed, and the **Pending uploads** dialog lets you:

- send a pending item right away with **Send now**;
- retry a failed item with **Try again**, copy its text with **Copy text**, or **Discard** it.

An item can fail because the server did not confirm it in time, validation rejected it, you lost permission, the linked record no longer exists or it conflicted with another change. The dialog tells you which. If an item expired, check whether it was already created before retrying so you do not create a duplicate.

## Related pages

- [Records and views](/_aphel/docs/using/records-and-views/): lists, filters, saved views and record pages.
- [Custom fields](/_aphel/docs/using/custom-fields/): fields, formulas, validation rules, layouts and custom objects.
- [Personal settings](/_aphel/docs/using/personal-settings/): your account, appearance and notifications.
- [Concepts](/_aphel/docs/start/concepts/): the model behind modules, blueprints and permissions.
