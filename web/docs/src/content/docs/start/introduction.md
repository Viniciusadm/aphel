---
title: Introduction
description: What Aphel is, who it is for, what you get when you install it, and how this documentation is organized.
sidebar:
  order: 1
---

Aphel is an open source platform for composing business systems from plugins. A small core provides identity, access control, records, events, background jobs and the plugin runtime. The core knows no business domain. Everything else, such as contacts, deals, tasks or quotes, is a module or a plugin that you can turn on or off.

A **blueprint** decides which modules, configuration and navigation make up a system. The system Aphel ships today is the `crm` blueprint and its variants for freelancers, agencies, real estate, consultancies, SaaS organizations and B2B sales teams. You can also start from a blank system and pick only the features you want.

The current version is 0.1.

## Who it is for

| You are | Start with |
| --- | --- |
| A company that wants a customer relationship system it fully controls | [Quickstart](/docs/start/quickstart/), then [Using Aphel](/docs/using/overview/) |
| An administrator running an installation | [Installation](/docs/admin/installation/) and [Configuration](/docs/admin/configuration/) |
| A developer who needs a custom field, rule, screen or integration | [Plugins overview](/docs/plugins/overview/) and [Your first plugin](/docs/plugins/first-plugin/) |
| An integrator who talks to Aphel from another system | [REST API](/docs/reference/rest-api/) and [Authentication](/docs/reference/authentication/) |

## What you get

- **One binary.** The backend is written in Rust. The React web application is embedded in the same executable, so one process serves both the interface and the API.
- **One installation, one company.** Each installation serves a single organization and has its own MySQL database. To serve another company, run another installation. There is no multi-tenancy and no organization switching.
- **MySQL 8.** All data lives in MySQL. Files go to local disk by default, or to S3-compatible storage if you configure it.
- **Blueprints.** At first start, a setup wizard asks about the business and chooses a blueprint, or lets you start blank. See [Blueprints](/docs/admin/blueprints/).
- **Modules.** Features compiled into the binary, such as people, deals, tasks, email, dashboards and automations. Administrators enable and disable them without restarting. See [Modules](/docs/admin/modules/).
- **Runtime plugins.** Third parties ship `.aphelplugin` packages (a WebAssembly backend plus an optional web bundle) that administrators install from the interface or the API, with no rebuild and no restart. See [Plugins overview](/docs/plugins/overview/).
- **Adaptable data model.** Custom fields, custom objects and relationships are configured on screen and work with filters, views, search, automations and the API. See [Custom fields](/docs/using/custom-fields/).
- **Access control.** Roles with permissions limited to own, team or organization records, plus per-field visibility. See [Roles and permissions](/docs/admin/roles-and-permissions/).
- **Reliable integration.** Every business change publishes a domain event and an audit entry in the same transaction. Events feed webhooks, automations and plugins. See [Events](/docs/plugins/events/) and [Webhooks and events](/docs/reference/webhooks-and-events/).
- **REST API.** Everything the interface does goes through `/api/v1`, documented as OpenAPI 3.1 at `/api/openapi.json` and browsable at `/api/docs`.
- **Interface.** English and Brazilian Portuguese, responsive, light and dark themes, installable as a progressive web app with limited offline support.

## What it is not

- It is not multi-tenant. One installation never holds data from two companies.
- It is not a hosted service. You run it yourself, with your own MySQL.
- It is not a collection of microservices. It is a modular monolith: one process for the HTTP server and optionally a second process for the worker, both from the same binary.
- Its core is not a CRM. The CRM is a blueprint built from modules, and a system without it is possible.

## License

Aphel is released under the GNU Affero General Public License, version 3. The repository includes the full text in the `LICENSE` file. If you modify Aphel and let users interact with it over a network, the AGPL requires you to offer them the corresponding source of your modified version.

## A tour of this documentation

| Section | What it covers |
| --- | --- |
| [Getting started](/docs/start/quickstart/) | Quickstart, the [mental model](/docs/start/concepts/), the [architecture](/docs/start/architecture/) and a [glossary](/docs/start/glossary/). |
| [Using Aphel](/docs/using/overview/) | Day-to-day use: [records and views](/docs/using/records-and-views/), [leads](/docs/using/leads/), [deals](/docs/using/deals-and-pipelines/), [tasks](/docs/using/tasks-and-calendar/), [email](/docs/using/email-and-sequences/), [dashboards](/docs/using/dashboards-and-reports/), [automations](/docs/using/automations/) and more. |
| [Administration](/docs/admin/installation/) | [Installing](/docs/admin/installation/), [configuring](/docs/admin/configuration/), [users and teams](/docs/admin/users-and-teams/), [security](/docs/admin/security/), [upgrading](/docs/admin/upgrading/), [backup and restore](/docs/admin/backup-and-restore/), [operations](/docs/admin/operations/) and the [command line](/docs/admin/cli/). |
| [Building plugins](/docs/plugins/overview/) | Everything needed to write a runtime plugin: the [manifest](/docs/plugins/manifest/), [hooks](/docs/plugins/hooks/), [events](/docs/plugins/events/), the [host API](/docs/plugins/host-api/), the [JavaScript](/docs/plugins/js-sdk/) and [Rust](/docs/plugins/rust-sdk/) SDKs, [web screens](/docs/plugins/web-screens/), [packaging](/docs/plugins/packaging/) and [examples](/docs/plugins/examples/). |
| [Reference](/docs/reference/rest-api/) | The [REST API](/docs/reference/rest-api/), [errors](/docs/reference/errors/), [permissions](/docs/reference/permissions-catalog/), [hooks](/docs/reference/hooks-catalog/), [design tokens](/docs/reference/design-tokens/) and the [module catalog](/docs/reference/module-catalog/). |

If you read only one page after this one, read [Concepts](/docs/start/concepts/). It explains the vocabulary the rest of the documentation relies on.
