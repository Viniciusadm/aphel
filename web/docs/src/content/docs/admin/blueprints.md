---
title: Blueprints
description: What a blueprint is, the eight blueprints that ship with Aphel, and how to read a blueprint.toml file.
sidebar:
  order: 6
---

A **blueprint** is a recipe for a system. It declares which modules an installation needs, which ones it recommends, how the navigation starts, which roles to create and which configuration packages to apply. The installer asks you to pick one on the Business step. You can also skip it and start from a blank system.

A blueprint is chosen once, at install time. It does not stay in control afterwards: you can enable or disable modules, edit roles and change the configuration freely. `GET /api/v1/blueprint` returns the blueprint the instance was installed from (key, name and navigation home), or `null` for a blank system.

## Built-in blueprints

Blueprints live in the `blueprints/` folder of the repository and are embedded in the binary. A blueprint is offered in the installer only if every module it names is compiled into the build.

All blueprints except `crm` extend `crm`, so they include everything in `crm` first.

| Key | Name | Adds on top of `crm` | Configuration package |
| --- | --- | --- | --- |
| `crm` | CRM | Required: `people`, `organizations`, `deals`, `notes`, `tagging`, `integrations`, `exports`, `org_export`. Recommended: `timeline`, `dashboard`, `reminders`, `favorites`, `followers`, `onboarding`, `imports`, `bulk_actions`, `duplicates`, `trash`, `privacy`, `marketplace`, `notifications`, `objects`, `search`, `saved_views`, `record_layouts`, `formulas`, `validation_rules`, `config_packages`, `calendar`, `tasks`, `workflow`, `money`, `documents`, `wasm_runtime` | `crm` |
| `freelancer` | Freelancer | Recommended: `booking`, `catalog_items`, `quotes`, `email`, `automations`, `demo`, `google`, `ai` | `crm`, then `freelancer` |
| `agency` | Agency | Recommended: `leads`, `catalog_items`, `line_items`, `deals_revenue`, `pricing`, `quotes`, `proposals`, `email`, `sequences`, `segments`, `automations`, `reports`, `booking`, `google`, `microsoft`, `ai`, `enrichment`, `esign` | `crm`, then `agency` |
| `consultancy` | Consultancy | Recommended: `booking`, `catalog_items`, `line_items`, `deals_revenue`, `quotes`, `proposals`, `approvals`, `email`, `reports`, `google`, `microsoft`, `ai`, `esign` | `crm` |
| `real_estate` | Real Estate | Recommended: `leads`, `assignment`, `forms`, `telephony`, `messaging`, `booking`, `email`, `automations`, `google`, `ai`, `enrichment`, `esign` | `crm`, then `real_estate` |
| `saas` | SaaS | Recommended: `leads`, `scoring`, `line_items`, `deals_revenue`, `renewals`, `goals`, `forecast`, `segments`, `automations`, `webhooks`, `insights`, `migrations_assistant`, `google`, `microsoft`, `sso`, `ai` | `crm`, then `saas` |
| `sales` | B2B Sales | Recommended: `leads`, `assignment`, `scoring`, `goals`, `forecast`, `reports`, `telephony`, `email`, `sequences`, `segments`, `automations`, `approvals`, `google`, `microsoft`, `sso`, `ai`, `enrichment` | `crm`, then `sales` |
| `other` | Other business | Recommended: `leads`, `catalog_items`, `quotes`, `email`, `automations`, `reports` | `crm` |

What the descriptions say about each variant:

- **Freelancer:** a lean pipeline for solo professionals, with hourly rate, estimated hours, delivery deadline and payment terms.
- **Agency:** new projects and contract renewals in separate workflows, with scope, contracted services and monthly retainer.
- **Real Estate:** sales and rental workflow, property details, rental fields shown only for rentals, showings, inspections and client profile tags.
- **SaaS:** from trial to closed subscription, with plan, MRR, seats, trial end and renewal date.
- **B2B Sales:** a consultative pipeline with qualification, demo, proposal and negotiation, and account industry, size and revenue.

The installer always installs the blueprint's **required** modules, together with their dependencies. It pre-selects the **recommended** modules that need no settings. You can add or remove recommended modules on the Features step. Recommended modules that need credentials, such as `google` or `esign`, are left for you to enable later. See [Integrations](/docs/admin/integrations/).

## Read a blueprint.toml

A blueprint is a TOML file in `blueprints/<key>/blueprint.toml`. This is the full shape, using the `crm` blueprint as the base:

```toml
key = "crm"
name = "CRM"
version = "1.0.0"
description = "Sales and customer relationship management."

[plugins]
required = ["people", "organizations", "deals"]
recommended = ["timeline", "dashboard"]

[navigation]
home = "/"

[configuration]
packages = ["crm"]
```

A child blueprint extends it:

```toml
key = "freelancer"
name = "Freelancer"
version = "1.0.0"
extends = "crm"

[plugins]
recommended = ["booking", "quotes"]
exclude = ["wasm_runtime"]

[configuration]
packages = ["freelancer"]
```

| Field | Meaning |
| --- | --- |
| `key` | Unique identifier. Lowercase letters, digits, `_` and `-`. |
| `name`, `version`, `description` | Shown in the installer. The description is optional. |
| `extends` | Key of a parent blueprint. Chains are resolved from the root down. An unknown parent or a cycle is an error. |
| `[plugins] required` | Modules that are always installed. |
| `[plugins] recommended` | Modules pre-selected in the installer. A module that is required is never also listed as recommended. |
| `[plugins] exclude` | Modules removed from the lists inherited from the parent chain, before this blueprint adds its own. |
| `[navigation] home` | Path of the home screen. |
| `[[navigation.sections]]` | Sidebar sections, each with a `key`, a `label` (a string, or a table of labels by language) and the `items` it contains. A section with the same key as an inherited one replaces its label and adds items. |
| `[[roles]]` | Roles created at install, each with a `key`, a `label` and a list of `permissions`. Permissions the build does not know are ignored, and the rest are granted at the organization scope. A role with the same key as an inherited one replaces its label and adds permissions. |
| `[configuration] packages` | Configuration packages applied right after installation. |

A navigation section and a role look like this:

```toml
[[navigation.sections]]
key = "sales"
label = { en = "Sales", pt-BR = "Vendas" }
items = ["deals", "people"]

[[roles]]
key = "account_executive"
label = "Account executive"
permissions = ["people.read", "deals.read"]
```

The blueprints that ship today define no roles and no navigation sections. Only `crm` sets the navigation home.

## Configuration packages of a blueprint

The names under `[configuration] packages` are the keys of the templates embedded in the binary. For each one, `blueprints/<package>/config/<language>.json` holds the package in `en` and `pt-BR`. The installer applies them after the modules are enabled, and creates the blueprint's roles first. If applying fails, the installation still completes, and the error is shown by `GET /api/v1/blueprint`. An administrator with `settings.manage` can apply them again with `POST /api/v1/blueprint/configuration`. Applying twice changes nothing, because packages reconcile by stable key. See [Configuration packages](/docs/admin/config-packages/).

:::note
The `blueprints/` directory is compiled into the binary. To add or change a blueprint you edit the files and rebuild. See [Installation](/docs/admin/installation/).
:::
