---
title: Concepts
description: The mental model of Aphel in plain language, from installations and blueprints to records, permissions, events, jobs and the API.
sidebar:
  order: 3
---

This page explains the ideas the rest of the documentation builds on. Each section ends with links to the pages that go deeper.

## The big picture

```text
                 one installation = one company = one MySQL database

  +---------------------------------------------------------------+
  | Blueprint   chooses modules, navigation, roles, configuration |
  +---------------------------------------------------------------+
  | Modules     built into the binary, enabled or disabled       |
  | Plugins     .aphelplugin packages installed at runtime       |
  +---------------------------------------------------------------+
  | Kernel      identity, access, records, events, jobs, files,   |
  |             audit, search, API, plugin host                   |
  +---------------------------------------------------------------+
```

The kernel is always on and knows no business domain. Everything a user recognizes as a feature comes from a module or a plugin. A blueprint is the recipe that picks them.

## Installation and organization

One installation serves one company, called **the organization**. The company's name, logo, color, language, time zone, currency and security policy are stored once for the whole installation. There is no organization switcher, no tenant identifier on the tables and no cross-company data.

To serve another company, run another installation with its own database. Users join an installation by invitation, and each user has exactly one role.

:::note
Do not confuse "the organization" (your company, one per installation) with the **organization record type**, which represents a customer company in the CRM. The record type is data inside the installation.
:::

The first administrator and the company are created only by the setup wizard (see the [Quickstart](/docs/start/quickstart/)).

## Blueprints

A **blueprint** is the identity of a system. It is a small file that lists:

- the modules that are **required** and **recommended**, and the ones to **exclude**;
- the navigation, including the home page;
- extra roles;
- the **configuration packages** applied after installation, for example the default sales pipeline and lost reasons.

A blueprint can extend another one. The shipped blueprints are `crm`, and `agency`, `consultancy`, `freelancer`, `real_estate`, `saas`, `sales` and `other`, all extending `crm`. The setup wizard reads the blueprint, enables its required modules, applies its configuration packages and remembers the choice. A blank system skips the blueprint and enables only what you pick.

"Required" is a property of the blueprint. No module is required by the platform itself: only the kernel is always on.

See [Blueprints](/docs/admin/blueprints/) and [Configuration packages](/docs/admin/config-packages/).

## Modules, runtime plugins and UI plugins

| | Built-in modules | Runtime plugins | UI plugins |
| --- | --- | --- | --- |
| Shipped as | Rust crates compiled into the binary | A `.aphelplugin` package | A runtime plugin that declares `[ui]` |
| Backend | Native Rust | WebAssembly, sandboxed | Optional |
| Installed by | Choosing what to compile | An administrator, from the interface or API, without rebuild or restart | Same as runtime plugins |
| Turned on and off | Settings, **Modules** | Settings, **Plugins** | Chosen by each user |
| Can add | Everything: routes, hooks, jobs, permissions, screens | Entities, fields, hooks, events, routes, schedules, settings, automation actions, widgets and web screens, within the permissions the administrator grants | A different interface (frame and record presentation) |

**Modules.** The build selection lives in a file (`aphel.modules.toml`); only the core and official modules are compiled. Disabling a module stops its screens, menus, widgets and routines for the whole organization, but keeps its data. A module can depend on others, and the platform blocks disabling a module that other enabled modules use.

**Runtime plugins.** A plugin package contains a `plugin.toml` manifest, a `backend.wasm` and an optional `web/` folder. Installing one shows a consent dialog that lists what it asks for. Every call a plugin makes runs with the permissions of the actor that triggered it, intersected with the capabilities the plugin declared. See [Plugins overview](/docs/plugins/overview/) and [Security model](/docs/plugins/security-model/).

**UI plugins.** A plugin that declares `[ui]` provides design tokens and optionally a web bundle with screens. It becomes an interface that every user can choose, with an organization default. See [UI plugins](/docs/plugins/ui-plugins/).

Governing rules for all of them: plugins depend on contracts, never on each other; each table has one owner; there are no foreign keys or SQL across plugins.

See [Modules](/docs/admin/modules/) and [Module catalog](/docs/reference/module-catalog/).

## Records, entities and record types

A **record** is one row of business data: a person, a deal, a task. Every record belongs to a **record type**, also called an **entity**. Record types are identified by a key:

| Kind | Key pattern | Example |
| --- | --- | --- |
| Provided by a module or plugin | `<module>.<entity>` | `people.person`, `deals.deal` |
| Created by an administrator | `custom.<key>` | `custom.vehicle` |

Every record type is registered in one **entity registry**. Lists, filters, sorting, search, import, export, bulk actions, automations and the API all work from the registry, so a new record type gains those features without extra code. Generic routes live at `/api/v1/entities/{entity}/records`.

### Custom fields

Administrators add **custom fields** to record types without changing the code. There are 14 built-in field types: text, long text, number, currency, date, date and time, select, multi-select, checkbox, URL, email, phone, user and record reference. Modules and plugins can register additional field types. Custom values are stored in typed, indexed columns, so they can be filtered and sorted like native fields.

### Custom objects and relationships

A **custom object** is a record type created entirely by an administrator, with its own fields, permissions and API. Up to 50 custom objects can exist per installation, each with up to 200 fields.

A **relationship** links two record types with a cardinality of one to one, one to many or many to many. Each end has its own label, and a relationship can be required. The `on_delete` behavior is either `restrict` (block deleting a record that still has links) or `unlink`.

See [Custom fields](/docs/using/custom-fields/), [Entities and fields](/docs/plugins/entities-and-fields/) and [Records API](/docs/reference/records-api/).

## Roles, permissions and scopes

Access control has three layers.

**Permissions.** A permission is a named capability such as `deals.read` or `settings.manage`. Modules and plugins add their own. Some permissions are administrative (they have no scope), and others are record permissions that carry a scope.

**Scopes.** A record permission is granted with one of three scopes:

| Scope | The user can reach |
| --- | --- |
| `own` | Records they own. |
| `team` | Records owned by members of their teams, including sub-teams. |
| `organization` | All records. |

**Roles.** A role is a set of permissions, each with a scope. Every installation starts with three system roles: `admin` (everything at organization scope, always full access, cannot be deleted), `manager` (defaults to team scope) and `member` (defaults to own scope). Administrators create custom roles. A user has one role and can belong to teams, which can be nested.

**Field permissions.** On top of record access, a role can have a field set to `hidden` or `read_only`. Hidden fields disappear from responses, forms and lists, and filtering or sorting by them is rejected with `field_not_accessible`. Administrators and locked fields are always editable.

Every use case receives an **access scope** (the user, role, teams and permissions) and checks it explicitly. Background work without a user runs as the system.

See [Roles and permissions](/docs/admin/roles-and-permissions/), [Users and teams](/docs/admin/users-and-teams/) and [Permissions catalog](/docs/reference/permissions-catalog/).

## Events, the outbox, hooks, automations and webhooks

These are four different ways to react to what happens in the system.

```text
 write request
      |
      v
 +---------------------------- one database transaction ---------+
 |  hooks (saving, validating, saved)   <- can change or veto    |
 |  write the record                                             |
 |  domain event -> outbox table        <- same transaction      |
 |  audit entry                                                  |
 +---------------------------------------------------------------+
      |  commit
      v
  committed hook                       (after commit, best effort)
      |
      v   worker, asynchronously
  outbox dispatch -> consumers -> automations, webhooks, notifications,
                                  search index, plugins
```

**Domain events.** Every business change publishes an event, such as `deal.amount_changed`, in the same transaction as the change. The event is stored in an **outbox** table, so it is never lost and never published for a change that was rolled back. A dispatcher in the worker then hands each event to every interested **consumer** with at-least-once delivery.

**Hooks.** A hook is a synchronous extension point inside the request. There are three kinds: an *action* reacts, a *veto* can block the operation by returning an error, and a *filter* can change the data passing through. Record hooks run at fixed points of the write pipeline. Hooks run inside the transaction, so they must be quick. See [Hooks](/docs/plugins/hooks/) and the [Hooks catalog](/docs/reference/hooks-catalog/).

**Automations.** A no-code feature for administrators: a **trigger** (a record event or a schedule), optional **conditions**, and a list of **actions** contributed by modules and plugins, such as creating or updating records. They run in the worker with the permissions of the person who published them. See [Automations](/docs/using/automations/).

**Webhooks.** Outgoing webhooks deliver selected public events to an external URL, signed and retried. Incoming webhooks give external systems a secret URL that can trigger actions. See [Integrations](/docs/admin/integrations/) and [Webhooks and events](/docs/reference/webhooks-and-events/).

## Jobs and the worker

A **job** is a unit of background work stored in a MySQL queue: sending an email, running an import, delivering a webhook, consuming an event. The **worker** (`aphel worker`, or `aphel serve --with-worker`) takes jobs from the queue, runs them with retries, and also runs the **scheduler**, which creates recurring jobs from cron expressions that include seconds.

Jobs may run more than once, so handlers must be idempotent. A web-only process without a worker still serves requests, but anything asynchronous stays queued.

See [Operations](/docs/admin/operations/) and [Schedules](/docs/plugins/schedules/).

## The API and API tokens

Everything the interface does uses the REST API at `/api/v1`. Browsers authenticate with a session cookie. Other systems use **API tokens**: a user creates a token with a name, scopes and an expiration, and sends it as `Authorization: Bearer`. A token can never do more than the user who created it can do at that moment: its permissions are its scopes intersected with the creator's current permissions.

Responses use a consistent envelope: `{ "data": ... }` for items, `{ "data": [...], "meta": { "total", "page", "per_page" } }` for lists, and `{ "error": { "code", "message", "details" } }` for errors.

See [REST API](/docs/reference/rest-api/), [Authentication](/docs/reference/authentication/), [Errors](/docs/reference/errors/) and [Pagination and filters](/docs/reference/pagination-and-filters/).

## Where to read next

| If you want to | Read |
| --- | --- |
| See how the pieces are built and wired | [Architecture](/docs/start/architecture/) |
| Look up a term | [Glossary](/docs/start/glossary/) |
| Choose or change a blueprint | [Blueprints](/docs/admin/blueprints/) |
| Write an extension | [Plugins overview](/docs/plugins/overview/) |
