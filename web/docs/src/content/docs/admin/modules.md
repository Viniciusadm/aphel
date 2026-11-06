---
title: Modules
description: Turn features on and off, configure them, and understand what disabling or removing a module does to your data.
sidebar:
  order: 5
---

Everything beyond the core is a **module**. Modules are compiled into the binary, and each installation decides which ones are turned on. Contacts, deals, tasks, email, quotes, automations, webhooks and single sign-on are all modules. The [module catalog](/docs/reference/module-catalog/) describes each one.

The module list in a build comes from `back/aphel.modules.toml`. `aphel modules list` prints the modules compiled into your binary. To add or remove modules from a build you recompile; to use them you enable them.

## States

| State | Meaning |
| --- | --- |
| Required | Always on and cannot be disabled. Only the `kernel` (identity, records engine, events, jobs and the module runtime) is required in the current build. |
| Enabled | Routes, screens, widgets, automation steps, event consumers and background jobs are active, and the module's permissions are in the role catalog. |
| Disabled | Its screens and routines are off. The data stays in the database. |
| Incompatible | The build cannot run the module: it needs a newer Aphel version, or a dependency is missing or has the wrong version. It cannot be enabled, and its pending migrations are skipped. |

## The Modules screen

Open Settings, Modules (permission `settings.manage`). Every member can read the list, because the web app loads the enabled modules from it. Each card shows the version, the minimum Aphel version, dependencies, the modules that depend on it, the permissions it adds and the events it publishes.

| Action | What happens |
| --- | --- |
| Enable | Requires every dependency to be enabled and every required setting to be filled. Applies the module's default permission grants, records the installed version and publishes `module.enabled`. |
| Disable | Hides the module's screens and stops its routes (they answer `404 module_disabled`), consumers, jobs, widgets and automation steps. Nothing is deleted, and enabling again restores everything. Refused when another enabled module depends on it (`module_required_by`) or when it is required (`module_required`). |
| Configure | Opens the module's settings. |
| Export data | Downloads every row the module keeps as JSON. |
| Remove data | Permanently deletes the module's data. See below. |

Changes reach every running process within about 5 seconds. You do not need to restart anything.

### Module settings

A module declares its settings, and the screen builds the form from them. Fields can be text, long text, URL, email, number (with a range), yes/no, a choice from a list, or a field of a record type. Required fields must be filled before the module can be enabled.

- Settings apply to the whole organization.
- **Secret** fields are encrypted with `APP_ENCRYPTION_KEY` and never returned. The form only shows whether a secret is configured. Leave it blank to keep it, or choose the remove option to clear it.
- Saving validates the values against the module's schema and returns the offending fields on error.
- The API is `PATCH /api/v1/modules/{key}/settings`.

Modules that connect to outside services keep their credentials here. See [Integrations](/docs/admin/integrations/).

## What disabling does not do

Disabling does not delete tables, records, files, settings or secrets. Records that belong to a disabled module stay in the database, and other modules keep their own references. Configuration package sections that belong to a disabled module are skipped when you plan or apply a package. See [Configuration packages](/docs/admin/config-packages/).

## Remove a module's data

Removing data is a separate, deliberate step:

1. Disable the module.
2. Export its data. The export must be made after the module was disabled.
3. Remove the data, typing the module key to confirm. This deletes the rows of the module's tables, its settings and its secrets, and publishes `module.data_purged`. It cannot be undone.

The command-line equivalent disables the module, runs its uninstall step and, with `--purge`, writes the export to a JSON file in the current directory before removing the data and reverting the module's schema:

```bash
aphel modules uninstall insights --purge
```

Without `--purge`, the data is kept. A required module cannot be uninstalled. See [CLI](/docs/admin/cli/).

## Choosing modules at install time

In the installer, the Features step lists the optional modules. Dependencies are selected automatically, modules that need configuration ask for it there, and the blueprint decides which ones are pre-selected. See [Installation](/docs/admin/installation/) and [Blueprints](/docs/admin/blueprints/).

## Inspect modules

| Command | Output |
| --- | --- |
| `aphel modules list` | Key, version, whether required, whether enabled (read from the database), compatibility and dependencies |
| `aphel modules doctor` | What each module contributes (routes, hooks, services, catalogs, worker parts) and the conflicts between modules |
| `GET /api/v1/modules` | The same information over the API, with settings and configured secrets for users with `settings.manage` |

The System screen also lists the compiled modules with their version, compatibility and migration state. See [Operations](/docs/admin/operations/).
