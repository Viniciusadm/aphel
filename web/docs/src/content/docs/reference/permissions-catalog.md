---
title: Permissions catalog
description: Every permission key of the platform and the built-in modules, what it allows, whether it is administrative or record-scoped, and which built-in roles receive it by default.
sidebar:
  order: 7
---

A permission is a string of the form `<resource>.<action>`. Roles hold sets of permissions, and the API authorizes each request by asking whether the caller holds the permission the route needs. This page lists the permissions defined by the platform and the modules built into the binary. Modules add their permissions only while they are enabled, and the live list for your installation is `GET /api/v1/permissions`.

## Two kinds of permission

| Kind | Meaning | Scope on a role |
| --- | --- | --- |
| Record-scoped | Acts on records of a type, such as `people.read` or `deals.update`. | A record scope: `own` (records the member owns), `team` (records owned by members of their teams) or `organization` (every record). Shares can widen it for individual records. |
| Administrative | Acts on the installation, such as `settings.manage` or `audit.read`. | Always `organization`: you either have it or you do not. |

The API reports the scope of each granted permission in `GET /api/v1/auth/me` under `permissions[]`. A missing permission answers `403 permission_denied`; a record outside the caller's scope is usually answered as if it did not exist (`404`).

## Built-in roles

An installation starts with three system roles. Administrators can create more roles and edit the grants of any role except the system administrator, in **Settings, Roles** (see [Roles and permissions](/_aphel/docs/admin/roles-and-permissions/)).

| Role key | Default grants |
| --- | --- |
| `admin` | Every permission in the catalog, at organization scope. Always includes the permissions of modules enabled later. |
| `manager` | Every record-scoped permission at `team` scope, plus the administrative permissions a module explicitly grants to managers. |
| `member` | Every record-scoped permission at `own` scope except the `.export` ones, plus the administrative permissions a module explicitly grants to members. |

A module can override this default for a permission by naming the roles that receive it. When it does, only those roles hold it, and an administrative permission is held at organization scope. The three right-hand columns of the table show the result: a dash means the role does not receive the permission by default.

## Catalog

| Permission | Module | Kind | What it allows | Admin | Manager | Member |
| --- | --- | --- | --- | --- | --- | --- |
| `settings.manage` | Platform | administrative | Change organization settings, fields, custom objects, pipelines-level configuration, integrations, webhooks and other administration screens. Also required by the trash restore and purge routes. | organization | - | - |
| `users.manage` | Platform | administrative | Invite, change and deactivate members, manage teams and roles, and see or revoke any API token. | organization | - | - |
| `audit.read` | Platform | administrative | Read the audit log of the organization. | organization | - | - |
| `system.manage` | Platform | administrative | Operations routes: system health, background jobs, failures, platform catalog and updates. | organization | - | - |
| `records.share` | Platform | administrative | Share any record you can read with users or teams, even when you do not own it. | organization | - | - |
| `ai.use` | AI assistant | administrative | Use the AI assistant: summaries, next actions, writing help and questions. | organization | - | - |
| `ai.view_usage` | AI assistant | administrative | See AI usage and the status of the AI provider. | organization | - | - |
| `approvals.manage` | Approvals | administrative | Manage approval policies. | organization | - | - |
| `assignment.manage` | Assignment | administrative | Manage lead distribution rules and see the assignment history. | organization | - | - |
| `automations.manage` | Automations | administrative | Create, publish, run and delete automations. | organization | - | - |
| `booking.manage` | Booking | record | Manage your booking links, availability and bookings. | organization | team | own |
| `calendar.read` | Calendar | administrative | Read calendar entries contributed by enabled modules. | organization | organization | organization |
| `catalog_items.read` | Products | record | Read products and services. | organization | organization | organization |
| `catalog_items.export` | Products | record | Export products and services. | organization | organization | - |
| `catalog_items.create` | Products | record | Create products and services. | organization | - | - |
| `catalog_items.update` | Products | record | Change products and services. | organization | - | - |
| `catalog_items.delete` | Products | record | Delete products and services. | organization | - | - |
| `deals.read` | Deals | record | Read deals. | organization | team | own |
| `deals.create` | Deals | record | Create deals. | organization | team | own |
| `deals.update` | Deals | record | Change deals, move them between stages and manage their team. | organization | team | own |
| `deals.delete` | Deals | record | Delete deals (move them to the trash). | organization | team | own |
| `deals.export` | Deals | record | Export deals. | organization | team | - |
| `deals.archive` | Deals | record | Archive and unarchive deals. | organization | team | own |
| `emails.read` | Email | record | Read e-mail conversations linked to records. | organization | team | own |
| `emails.send` | Email | record | Send e-mail from records and manage personal e-mail templates and tracking settings. | organization | team | own |
| `enrichment.use` | CNPJ enrichment | administrative | Look up and apply company data by CNPJ. | organization | - | - |
| `esign.send` | Electronic signature | administrative | Send documents for electronic signature and cancel requests. | organization | - | - |
| `esign.view` | Electronic signature | administrative | See signature requests and their status. | organization | - | - |
| `forms.manage` | Lead forms | administrative | Create and manage public lead forms. | organization | - | - |
| `goals.manage` | Goals | record | Create and see sales goals. | organization | team | own |
| `google.calendar_sync` | Google | record | Connect a Google calendar and sync meetings. | organization | team | own |
| `lifecycle.manage` | Leads | administrative | Manage lifecycle settings and disqualification reasons. | organization | - | - |
| `line_items.read` | Line items | administrative | Read the line items settings and totals policy. | organization | organization | organization |
| `messaging.view` | Messaging channels | administrative | See shared inbox channels and conversations. | organization | - | - |
| `messaging.reply` | Messaging channels | administrative | Reply to, assign and link conversations. | organization | - | - |
| `messaging.manage_channels` | Messaging channels | administrative | Configure messaging channels. | organization | - | - |
| `microsoft.calendar_sync` | Microsoft | record | Connect an Outlook calendar and sync meetings. | organization | team | own |
| `money.read` | Money | administrative | Read enabled currencies and exchange rates. | organization | organization | organization |
| `notes.read` | Notes | record | Read notes attached to records. | organization | team | own |
| `notes.create` | Notes | record | Create notes. | organization | team | own |
| `notes.update` | Notes | record | Edit notes. | organization | team | own |
| `notes.delete` | Notes | record | Delete notes. | organization | team | own |
| `organization.export` | Organization backup | administrative | Create and download full backups of the organization. | organization | - | - |
| `organizations.read` | Organizations | record | Read organizations. | organization | team | own |
| `organizations.create` | Organizations | record | Create organizations. | organization | team | own |
| `organizations.update` | Organizations | record | Change organizations. | organization | team | own |
| `organizations.delete` | Organizations | record | Delete organizations (move them to the trash). | organization | team | own |
| `organizations.export` | Organizations | record | Export organizations. | organization | team | - |
| `organizations.archive` | Organizations | record | Archive and unarchive organizations. | organization | team | own |
| `people.read` | People | record | Read people. | organization | team | own |
| `people.create` | People | record | Create people. | organization | team | own |
| `people.update` | People | record | Change people and link them to organizations. | organization | team | own |
| `people.delete` | People | record | Delete people (move them to the trash). | organization | team | own |
| `people.export` | People | record | Export people. | organization | team | - |
| `people.archive` | People | record | Archive and unarchive people. | organization | team | own |
| `privacy.manage` | Privacy | administrative | Manage data subject requests, retention policies and communication opt-outs. | organization | - | - |
| `proposals.manage` | Proposals | administrative | Manage proposal templates and preview them. | organization | - | - |
| `proposals.generate` | Proposals | administrative | Generate PDF proposals from deals. | organization | - | - |
| `quotes.read` | Quotes | record | Read quotes. | organization | team | own |
| `quotes.create` | Quotes | record | Create quotes. | organization | team | own |
| `quotes.update` | Quotes | record | Change quotes and their items. | organization | team | own |
| `quotes.delete` | Quotes | record | Delete quotes. | organization | team | own |
| `quotes.send` | Quotes | record | Send quotes to customers. | organization | team | own |
| `scoring.manage` | Scoring | administrative | Manage lead scoring models. | organization | - | - |
| `sequences.read` | Sequences | record | Read sequences and enrollments. | organization | team | own |
| `sequences.enroll` | Sequences | record | Enroll contacts in sequences and pause, resume or end enrollments. | organization | team | own |
| `sequences.manage` | Sequences | administrative | Create and change sequences. | organization | - | - |
| `tasks.read` | Tasks | record | Read tasks and activities. | organization | team | own |
| `tasks.create` | Tasks | record | Create tasks and activities. | organization | team | own |
| `tasks.update` | Tasks | record | Change, complete and reopen tasks. | organization | team | own |
| `tasks.delete` | Tasks | record | Delete tasks. | organization | team | own |
| `tasks.export` | Tasks | record | Export tasks. | organization | team | - |
| `workflow.read` | Workflow | administrative | Read pipelines, stages and lost reasons. | organization | organization | organization |

Notes on the table:

- A role that holds a record-scoped permission only reaches the records its scope allows; for example a member with `deals.read` at `own` scope reads only their own deals.
- Routes that list `settings.manage` as the requirement cover most configuration screens of the modules, such as webhooks, custom objects and custom fields.
- Some modules authorize additional actions with the permissions of the record type they act on, for example bulk exports use `<type>.export`, bulk actions use `<type>.update`, `<type>.archive` and `<type>.delete`, and attachments use the update permission of the record.
- Plugin-provided and compiled-in modules that are not in the table add their own permissions to the same catalog.

## Custom object permissions

Each custom object adds six record-scoped permissions, named after its key:

| Permission | What it allows |
| --- | --- |
| `custom.<key>.read` | Read records of the object. |
| `custom.<key>.create` | Create records. |
| `custom.<key>.update` | Change records. |
| `custom.<key>.delete` | Delete records (move them to the trash). |
| `custom.<key>.archive` | Archive and unarchive records. |
| `custom.<key>.export` | Export records. |

When an administrator creates the object, administrators receive all of them at organization scope, managers receive them with `team` scope and members with `own` scope, and members do not receive `export`. Roles can be adjusted afterwards.

## Permissions declared by plugins

Runtime plugins declare their own permissions in the manifest as `<plugin key>.<action>`, with default grants for roles, and they can request role presets that an administrator chooses to create at install time. Entities declared by a plugin get the same six permissions as custom objects. These permissions enter the catalog while the plugin is installed. See [Permissions and roles](/_aphel/docs/plugins/permissions-and-roles/) for how to declare them.

## API token scopes

Permissions map to the scopes of [API tokens](/_aphel/docs/reference/authentication/#scopes) by their resource: `<resource>.read` becomes `<resource>:read`, and every other action becomes `<resource>:write`. For example `deals.read` gives `deals:read`, and `deals.create`, `deals.update`, `deals.delete`, `deals.archive` and `deals.export` all give `deals:write`. The administrative permissions follow the same rule, so `settings.manage` becomes `settings:write` and `audit.read` becomes `audit:read`.
