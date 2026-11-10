---
title: Configuration packages
description: Export, validate, plan and apply portable configuration between installations, without touching business data.
sidebar:
  order: 7
---

A **configuration package** is a JSON file that carries how an organization is set up, never its data. You can export the configuration of one installation and apply it to another, install a ready-made template, or keep a copy as a baseline. The feature belongs to the `config_packages` module and needs the `settings.manage` permission.

Open Settings, Configuration packages. It has three tabs: Export, Install and History.

## What a package carries

A package is a JSON object with the format `aphel-config`:

```json
{
  "format": "aphel-config",
  "format_version": 1,
  "min_aphel_version": "0.1.0",
  "name": "Agency",
  "description": "For marketing, design and development agencies.",
  "author": "Aphel",
  "locale": "en",
  "sections": {
    "fields": [],
    "workflow": []
  }
}
```

| Field | Meaning |
| --- | --- |
| `format`, `format_version` | Must be `aphel-config` and a version this Aphel reads (currently up to 1). |
| `min_aphel_version` | The oldest Aphel that can apply the package. A newer requirement is rejected (`aphel_too_old`). |
| `id`, `version` | Optional. A package with both can be installed and updated as a versioned package. |
| `name` | Required, 1 to 140 characters. |
| `description`, `author` | Optional, up to 2000 and 140 characters. |
| `locale` | `en` or `pt-BR`. |
| `sections` | The content, grouped by section. |

A package can be at most 2 MB, and unknown top-level keys are rejected.

### Sections

Each section is owned by a module. If the module is disabled, the section is skipped when you plan or apply. `GET /api/v1/config-packages/sections` lists the sections the installation can carry, in installation order. The build includes these keys:

| Section | Carries |
| --- | --- |
| `fields` | Custom fields and their options |
| `workflow` | Pipelines and stages |
| `lost_reasons` | Lost reasons |
| `stage_guidance` | Guidance shown on stages |
| `task_types` | Activity types |
| `deal_member_roles` | Deal team roles |
| `currencies` | Enabled currencies and rates |
| `tags` | Tags |
| `layouts` | Record form layouts |
| `views` | Organization saved views |
| `dashboard` | The default dashboard |
| `custom_objects` | Custom record types |
| `validation_rules` | Validation rules |
| `lifecycle` | Lifecycle transitions and disqualification reasons |
| `scoring_models` | Lead scoring models |
| `assignment_rules` | Lead distribution rules |
| `approval_policies` | Approval policies |
| `automations` | Automations |
| `sequences` | Email sequences |
| `email_templates` | Email templates |
| `proposals.templates` | Proposal templates |
| `price_lists`, `discount_limits` | Price lists and discount limits |
| `line_items` | Line item settings |
| `forecast` | Forecast settings |
| `pipeline_renewals` | Renewal pipelines |
| `call_outcomes` | Call outcomes |

## Stable keys

Items in a package are referenced by **stable keys**, never by database id. A key is an identifier of up to 64 characters. When you export an item that has no key yet, Aphel derives one from its name (and adds a numeric suffix if it clashes), and it keeps that key from then on. When a package is applied, items are matched by key:

- A key that does not exist yet creates the item.
- A key that exists is compared with the package and updated if it differs.

This makes packages **idempotent**: applying the same package twice changes nothing the second time. It also means you can edit an exported package by hand, as long as you keep the keys.

## Export

On the Export tab, choose the sections and optionally a name, description and author, then download the file. Without a selection, all sections are exported. The API is `POST /api/v1/config-packages/export` with optional `sections`, `name`, `description`, `author` and `locale`.

An export contains only portable settings. No contact, company or deal is exported.

## Validate, plan and apply

Installing is a two-step flow so that you always see what will change before anything is written.

1. **Plan.** Send the package (or a template key) to `POST /api/v1/config-packages/plan`. Nothing is written. The answer says whether the package is valid, with `issues` that have paths such as `sections.fields.0.type`. When it is valid, it lists one item per field, pipeline, stage, lost reason, activity type, tag, layout, view and dashboard with an action, and a `fingerprint`:

   | Action | Meaning |
   | --- | --- |
   | `create` | The key does not exist yet |
   | `update` | The key exists and differs |
   | `unchanged` | The item already matches |
   | `skip_conflict` | The change cannot be applied as is, for example because the installation holds data that conflicts. It carries a `conflict` with a code and message, and it is skipped unless it is resolvable and you decide `apply` |

   Sections of disabled modules are listed in `skipped_sections`. References to fields of a disabled module (in a layout, a view column or sort, a stage requirement) are removed before validation and listed in `skipped_fields`. A view whose filter uses such a field is left out.

2. **Apply.** Send the same package or template together with the `fingerprint` to `POST /api/v1/config-packages/apply`. Optionally add `decisions`, a map from conflicting item to `apply` or `skip`. The plan is applied in one transaction. If the organization changed after the plan, Aphel answers `409 plan_outdated` and you plan again. The application publishes `config_package.applied`, writes an audit entry and records the installation in the history.

In the app, the Install tab accepts an uploaded `.json` file (up to 2 MB) or a ready-made template, shows the plan, and asks you to confirm.

## Templates

Templates are packages embedded in the binary, one set per blueprint (`crm`, `freelancer`, `agency`, `real_estate`, `saas`, `sales`), each in `en` and `pt-BR`. `GET /api/v1/config-packages/templates` lists them, and `GET /api/v1/config-packages/templates/{key}` returns one. If the language you ask for is not available, another one is used. The installer applies the template of the chosen blueprint automatically. See [Blueprints](/_aphel/docs/admin/blueprints/).

## History

`GET /api/v1/config-packages/installs` lists every installation, newest first, and `GET /api/v1/config-packages/installs/{id}` returns one with the package that was applied.

## Versioned packages

A package that carries an `id` and a `version` can be installed through the versioning endpoints, which keep a snapshot of each item:

- `GET /api/v1/config-package-versions` lists the installed versions.
- `POST /api/v1/config-package-versions/compare` compares a package version with the installed one. Each item gets a status (`added`, `changed`, `removed`, `unchanged`), whether it was changed locally since installation, and the resulting plan. Items changed both by the package and locally are conflicts that need a decision.
- `POST /api/v1/config-package-versions/apply` installs or updates the package. Conflicts follow `resolutions`, and the default `keep_local` keeps your local version of the item.

## What installing never does

- It **never deletes** anything. Items missing from a package are left alone.
- It **never touches business records**: contacts, companies, deals, notes, files and users are not read or written.
- It never carries or changes secrets and credentials.
- It never applies sections of modules that are disabled.
- It never writes before you confirm: planning is read-only, and applying requires the plan fingerprint.

For a full copy of an installation including its records, use a full export instead. See [Backup and restore](/_aphel/docs/admin/backup-and-restore/).
