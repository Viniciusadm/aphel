---
title: Module catalog
description: Every module built into Aphel with its key, layer, purpose, permissions, dependencies, web package and the blueprints that enable it.
sidebar:
  order: 10
---

Aphel is a modular monolith. The binary is built from the **kernel** plus the modules selected in `back/aphel.modules.toml`, and each installation enables the ones it needs. This page lists the modules shipped in the repository. For how modules are enabled and removed see [Modules](/_aphel/docs/admin/modules/), and for how blueprints pick them see [Blueprints](/_aphel/docs/admin/blueprints/).

## How to read an entry

| Field | Meaning |
| --- | --- |
| Key | The stable identifier used in `aphel.modules.toml`, in `aphel modules list` and in the `x-aphel-module` extension of the OpenAPI document. |
| Layer | `capability` modules offer a generic ability (people, tasks, workflow, search) and know no business domain; `domain` modules add business behavior on top of capabilities. A capability module can require only other capabilities; a domain module can require capabilities and other domain modules. |
| Depends on | Modules that must be enabled for this one to work. Enabling a module needs its dependencies first, and disabling one is refused while others depend on it. |
| Optional dependencies | Modules this one integrates with when they are enabled, and works without. They are loaded before it. |
| Permissions | Permission keys the module registers; see the [permissions catalog](/_aphel/docs/reference/permissions-catalog/). |
| Web package | Whether the module has its own screens in the web interface (a package under `web/src/modules`). |
| Blueprints | Whether the blueprints name the module as required (enabled by the installer) or recommended (offered). Every blueprint other than `crm` extends `crm` and inherits its lists. |

No module except the kernel is required: the kernel is always on, and every other module can be disabled in a blank system. Modules can also be disabled by a runtime plugin that provides the same contract.

## Kernel

The kernel is part of every build and cannot be disabled. It owns the host (HTTP, security, sessions, API tokens), identity and access (users, roles, teams), the entity, query and record pipeline, custom fields, files, the audit log, the installer and the system administration routes.

| | |
| --- | --- |
| Key | `kernel` |
| Required | yes, always enabled |
| Permissions | `settings.manage`, `users.manage`, `audit.read`, `system.manage`, `records.share` |
| Web package | yes (`kernel`) |

## Capability modules

### AI assistant (`ai`)

Summaries, next-step suggestions, a writing assistant and natural language queries with a configurable AI provider.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | `deals`, `tasks` |
| Permissions | `ai.use`, `ai.view_usage` |
| Web package | yes |
| Blueprints | Recommended by `agency`, `consultancy`, `freelancer`, `real_estate`, `saas`, `sales`. |

### Approvals (`approvals`)

Approval policies (record type, moment, condition, sequential steps), approval requests with decisions and the approval gate of record changes.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | `approvals.manage` |
| Web package | yes |
| Blueprints | Recommended by `consultancy`, `sales`. |

### Assignment (`assignment`)

Record assignment rules (round robin, weighted, least loaded, routing by field).

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | `assignment.manage` |
| Web package | yes |
| Blueprints | Recommended by `real_estate`, `sales`. |

### Automations (`automations`)

No-code automations: triggers, conditions and actions over records, versions, dry runs, time triggers, waits and run history.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | `automations.manage` |
| Web package | yes |
| Blueprints | Recommended by `agency`, `freelancer`, `other`, `real_estate`, `saas`, `sales`. |

### Booking (`booking`)

Booking links: availability, meeting types, team links and the public booking page.

| | |
| --- | --- |
| Layer | capability |
| Depends on | `tasks`, `people` |
| Optional dependencies | `assignment` |
| Permissions | `booking.manage` |
| Web package | yes |
| Blueprints | Recommended by `agency`, `consultancy`, `freelancer`, `real_estate`. |

### Bulk actions (`bulk_actions`)

Bulk actions over selected records or every record of a filter, with progress and report.

| | |
| --- | --- |
| Layer | capability |
| Uses contracts | `tagging@1` |
| Depends on | none |
| Optional dependencies | `exports` |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Calendar (`calendar`)

Calendar entries contributed by enabled record owners.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | `calendar.read` |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Calls (`telephony`)

Call logging as tasks, call outcomes, call sessions, telephony providers and the calls report.

| | |
| --- | --- |
| Layer | capability |
| Depends on | `tasks` |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by `real_estate`, `sales`. |

### CNPJ enrichment (`enrichment`)

Looks up public company data by CNPJ (BrasilAPI and ReceitaWS) and fills the chosen fields, recording the source of each value. Always triggered manually unless the automation is enabled.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | `deals`, `tasks` |
| Permissions | `enrichment.use` |
| Web package | yes |
| Blueprints | Recommended by `agency`, `real_estate`, `sales`. |

### Configuration packages (`config_packages`)

Export, validate, plan and apply portable configuration packages of the organization.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Dashboard (`dashboard`)

Home dashboard: widget catalog and data, per-user layout and the organization default layout.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Demo (`demo`)

Fictional demo data (seed, removal and reset) and the public demo mode status.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | `onboarding`, `money`, `workflow`, `catalog_items`, `pricing`, `line_items`, `quotes`, `approvals` |
| Permissions | none |
| Web package | no |
| Blueprints | Recommended by `freelancer`. |

### Documents (`documents`)

Shared document rendering, layout and owner-authorized sources.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | no |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Duplicates (`duplicates`)

Duplicate detection rules, the duplicate queue and record merging.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | `leads`, `trash` |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Electronic signature (`esign`)

Sends PDFs attached to deals for electronic signature on Clicksign and tracks their status through webhooks. It has no legal validity of its own: validity comes from the provider.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | `deals`, `tasks` |
| Permissions | `esign.send`, `esign.view` |
| Web package | yes |
| Blueprints | Recommended by `agency`, `consultancy`, `real_estate`. |

### Email (`email`)

Personal mailboxes, synchronization, conversations linked to records, sending, templates and tracking.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | `emails.read`, `emails.send` |
| Web package | yes |
| Blueprints | Recommended by `agency`, `consultancy`, `freelancer`, `other`, `real_estate`, `sales`. |

### Exports (`exports`)

CSV and XLSX exports of filtered records, the generated file helpers and the worker state shared by the data modules.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Required by every blueprint (inherited from `crm`). |

### Favorites (`favorites`)

Personal favorites: pinned records and saved views shown in the sidebar.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Followers (`followers`)

Following registered records and owner-provided events.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Formulas (`formulas`)

Formula fields that compute a value from other fields of a record.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Goals (`goals`)

Sales goals per user, team or organization (won amount, won deals, completed tasks, new people) and their progress.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | `deals` |
| Permissions | `goals.manage` |
| Web package | yes |
| Blueprints | Recommended by `saas`, `sales`. |

### Google (`google`)

Sign-in with Google, meeting sync with Google Calendar and Gmail mailboxes.

| | |
| --- | --- |
| Layer | capability |
| Depends on | `sso` |
| Optional dependencies | none |
| Permissions | `google.calendar_sync` |
| Web package | yes |
| Blueprints | Recommended by `agency`, `consultancy`, `freelancer`, `real_estate`, `saas`, `sales`. |

### Imports (`imports`)

CSV and XLSX imports with mapping, dry run, execution and retry of failed rows over the record type handlers.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Integrations (`integrations`)

Integration center: providers (generic webhook and module providers), configuration, connection test and activation.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Required by every blueprint (inherited from `crm`). |

### Lead forms (`forms`)

Public forms that create people and deals.

| | |
| --- | --- |
| Layer | capability |
| Depends on | `people` |
| Optional dependencies | `leads`, `deals`, `tasks`, `assignment` |
| Permissions | `forms.manage` |
| Web package | yes |
| Blueprints | Recommended by `real_estate`. |

### Line items (`line_items`)

Quantity, pricing, discounts and totals attached to records.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | `line_items.read` |
| Web package | yes |
| Blueprints | Recommended by `agency`, `consultancy`, `saas`. |

### Marketplace (`marketplace`)

Catalog of installable plugins and configuration packages.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Media library (`media`)

A library of images and files (`media.asset`) with name, alternative text, caption, credit and focal point. Choosing an asset in an editor attaches the same file to the record that uses it, so nothing is uploaded twice and the public site serves it as that record's image. `GET /api/v1/media/assets/{id}/usage` lists the records that use an asset, and deleting an asset in use is refused (`media_asset_in_use`). Editors with `media.assets.manage` send their uploads through the library.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | `media.assets.read`, `media.assets.manage` |
| Web package | yes |
| Blueprints | Recommended by `agency`, `other` and `retail`. |

### Messaging channels (`messaging`)

Shared inbox for WhatsApp Cloud API and email (IMAP/SMTP), with conversations on the contact timeline.

| | |
| --- | --- |
| Layer | capability |
| Depends on | `people` |
| Optional dependencies | `deals`, `tasks` |
| Permissions | `messaging.view`, `messaging.reply`, `messaging.manage_channels` |
| Web package | yes |
| Blueprints | Recommended by `real_estate`. |

### Microsoft (`microsoft`)

Sign-in with a Microsoft account (Entra ID), meeting sync with Outlook and Outlook mailboxes.

| | |
| --- | --- |
| Layer | capability |
| Depends on | `sso` |
| Optional dependencies | none |
| Permissions | `microsoft.calendar_sync` |
| Web package | yes |
| Blueprints | Recommended by `agency`, `consultancy`, `saas`, `sales`. |

### Money (`money`)

Enabled currencies of the organization, dated exchange rates and the conversion of amounts to the base currency.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | `money.read` |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Notes (`notes`)

Notes attached to records, with mentions and private notes.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | `notes.read`, `notes.create`, `notes.update`, `notes.delete` |
| Web package | yes |
| Blueprints | Required by every blueprint (inherited from `crm`). |

### Notifications (`notifications`)

In-app and e-mail notifications and each user's delivery preferences.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Objects (`objects`)

Administration of custom objects and the relationships between record types.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Organization backup (`org_export`)

Full export of the organization (zip with records, relationships, files, configuration and manifest) and its restoration.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | `organization.export` |
| Web package | yes |
| Blueprints | Required by every blueprint (inherited from `crm`). |

### Organizations (`organizations`)

Companies, their hierarchy (company groups) and group totals.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | `organizations.read`, `organizations.create`, `organizations.update`, `organizations.delete`, `organizations.export`, `organizations.archive` |
| Web package | yes |
| Blueprints | Required by every blueprint (inherited from `crm`). |

### People (`people`)

Contacts and their links to organizations.

| | |
| --- | --- |
| Layer | capability |
| Depends on | `organizations` |
| Optional dependencies | none |
| Permissions | `people.read`, `people.create`, `people.update`, `people.delete`, `people.export`, `people.archive` |
| Web package | yes |
| Blueprints | Required by every blueprint (inherited from `crm`). |

### Price lists (`pricing`)

Price lists by currency, segment or company with validity periods and quantity tiers, product bundles and the maximum discount per role.

| | |
| --- | --- |
| Layer | capability |
| Depends on | `catalog_items` |
| Optional dependencies | `organizations`, `segments` |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by `agency`. |

### Privacy (`privacy`)

Privacy (LGPD): personal data subjects, data subject requests and retention policies.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | `privacy.manage` |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Products (`catalog_items`)

Catalog of products and services.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | `catalog_items.read`, `catalog_items.export`, `catalog_items.create`, `catalog_items.update`, `catalog_items.delete` |
| Web package | yes |
| Blueprints | Recommended by `agency`, `consultancy`, `freelancer`, `other`. |

### Record layouts (`record_layouts`)

Layouts of the create, edit and detail screens of record types.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Reminders (`reminders`)

Reminders before a task is due, the morning notice of tasks due today and standalone reminders linked to a record.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | `tasks` |
| Permissions | none |
| Web package | no |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Reports (`reports`)

Saved entity reports, exports and e-mail schedules.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by `agency`, `consultancy`, `other`, `sales`. |

### Saved views (`saved_views`)

Saved views of record lists, with their filters, sorting and columns.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Scoring (`scoring`)

Lead scoring models (profile and behavior criteria, decay, bands) and the scores of a record with their explanation.

| | |
| --- | --- |
| Layer | capability |
| Depends on | `people`, `organizations` |
| Optional dependencies | none |
| Permissions | `scoring.manage` |
| Web package | yes |
| Blueprints | Recommended by `saas`, `sales`. |

### Search (`search`)

Global search across record types.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Segments (`segments`)

Named dynamic filters of records, usable as the in_saved_filter filter condition.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by `agency`, `saas`, `sales`. |

### Sequences (`sequences`)

Sales sequences (cadences): steps, enrollments, automatic exits, sending limits and per-step metrics.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | `email`, `people`, `tasks` |
| Permissions | `sequences.read`, `sequences.enroll`, `sequences.manage` |
| Web package | yes |
| Blueprints | Recommended by `agency`, `sales`. |

### Setup assistant (`onboarding`)

First-run setup assistant: questions, template recommendation and state.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Single sign-on (OIDC) (`sso`)

Single sign-on with any corporate OpenID Connect provider (Okta, Keycloak, Auth0, Entra ID…).

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by `saas`, `sales`. |

### Tags (`tagging`)

Tags of the organization and tagging of opted-in records.

| | |
| --- | --- |
| Layer | capability |
| Provides contracts | `tagging@1` |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Required by every blueprint (inherited from `crm`). |

### Tasks (`tasks`)

Tasks, logged tasks, activity types, calendar and activity inbox.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | `tasks.read`, `tasks.create`, `tasks.update`, `tasks.delete`, `tasks.export` |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Timeline (`timeline`)

Owner-contributed record timeline.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Trash (`trash`)

Deleted records: restore, permanent deletion and retention.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### Validation rules (`validation_rules`)

Validation rules that block record writes that break a condition.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

### WebAssembly runtime (`wasm_runtime`)

Runtime plugins: install .aphelplugin packages (WebAssembly backend and web bundle) without rebuilding or restarting Aphel.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Required: always enabled in every installation and not listed among the modules a company or operator can toggle. |

### Webhooks (`webhooks`)

Outgoing webhooks over the open catalog of public events (signed deliveries, retries, redelivery) and incoming webhooks that create or update records.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by `saas`. |

### Workflow (`workflow`)

Workflow stages (probability, rotting, required fields, guidance and checklist) and lost reasons.

| | |
| --- | --- |
| Layer | capability |
| Depends on | none |
| Optional dependencies | none |
| Permissions | `workflow.read` |
| Web package | yes |
| Blueprints | Recommended by every blueprint (inherited from `crm`). |

## Domain modules

### Deal revenue (`deals_revenue`)

Recurring revenue projections for deals.

| | |
| --- | --- |
| Layer | domain |
| Depends on | `deals`, `line_items` |
| Optional dependencies | none |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by `agency`, `consultancy`, `saas`. |

### Deals (`deals`)

Deals, stage changes, the Kanban board and the team of each deal.

| | |
| --- | --- |
| Layer | domain |
| Depends on | `money`, `workflow` |
| Optional dependencies | none |
| Permissions | `deals.read`, `deals.create`, `deals.update`, `deals.delete`, `deals.export`, `deals.archive` |
| Web package | yes |
| Blueprints | Required by every blueprint (inherited from `crm`). |

### Extra widgets (`insights`)

Additional dashboard widgets: deals by source, team activity ranking and birthdays of the month.

| | |
| --- | --- |
| Layer | domain |
| Depends on | none |
| Optional dependencies | `deals`, `tasks` |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by `saas`. |

### Forecast (`forecast`)

Revenue forecast per month from open deals, with the organization method, categories and horizon.

| | |
| --- | --- |
| Layer | domain |
| Depends on | none |
| Optional dependencies | `deals` |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by `saas`, `sales`. |

### Leads (`leads`)

Lifecycle stages of people and organizations, automatic transitions, the lead inbox (qualify, disqualify, convert) and disqualification reasons.

| | |
| --- | --- |
| Layer | domain |
| Depends on | `people`, `organizations` |
| Optional dependencies | none |
| Permissions | `lifecycle.manage` |
| Web package | yes |
| Blueprints | Recommended by `agency`, `other`, `real_estate`, `saas`, `sales`. |

### Migration assistant (`migrations_assistant`)

Assisted migration from other CRMs: recognized columns, conversion of their CSV exports and pre-filled imports.

| | |
| --- | --- |
| Layer | domain |
| Depends on | none |
| Optional dependencies | `imports`, `tasks`, `notes` |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by `saas`. |

### Proposals (`proposals`)

Document templates and PDF proposals generated from deals.

| | |
| --- | --- |
| Layer | domain |
| Depends on | `documents` |
| Optional dependencies | none |
| Permissions | `proposals.manage`, `proposals.generate` |
| Web package | yes |
| Blueprints | Recommended by `agency`, `consultancy`. |

### Quotes (`quotes`)

Quotes of deals: versions, items, discounts and taxes, approval, sending, the signed public page and acceptance.

| | |
| --- | --- |
| Layer | domain |
| Depends on | `documents`, `workflow` |
| Optional dependencies | none |
| Permissions | `quotes.read`, `quotes.create`, `quotes.update`, `quotes.delete`, `quotes.send` |
| Web package | yes |
| Blueprints | Recommended by `agency`, `consultancy`, `freelancer`, `other`. |

### Renewals (`renewals`)

Automatic renewal of won deals whose contract is ending, configured per pipeline.

| | |
| --- | --- |
| Layer | domain |
| Depends on | `workflow` |
| Optional dependencies | `deals`, `workflow`, `line_items` |
| Permissions | none |
| Web package | yes |
| Blueprints | Recommended by `saas`. |

## Compiling the selection

The selection of modules compiled into a build lives in `back/aphel.modules.toml`, one line per module (`<key> = { path = "modules/<key>" }`, or a git tag or a version), and `make modules` generates the registries from it. The same file lists the blueprints included in the build. Only the core and the official modules are compiled; third-party extensions are runtime plugins (see [Plugins overview](/_aphel/docs/plugins/overview/)).
