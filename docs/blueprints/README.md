# Blueprints

A blueprint is the identity of a system. It contains no code: one folder `blueprints/<key>/` with a `blueprint.toml`, plus configuration packages and themes beside it. The installer reads blueprints; a blank system is the absence of one.

```toml
key = "agency"
name = "Agency"
version = "1.0.0"
description = "..."
extends = "crm"

[plugins]
required = ["people", "organizations"]
recommended = ["booking", "reports"]
exclude = ["renewals"]

[navigation]
home = "/people"
sections = [{ key = "work", label = "Work", items = ["people", "tasks"] }]

[[roles]]
key = "manager"
label = "Manager"
permissions = ["people.read"]

[configuration]
packages = ["crm"]
theme = "themes/default.json"
```

## Resolution (`extends`)

The `aphel-blueprint` crate (`back/blueprint`) parses every `blueprints/*/blueprint.toml` (embedded in the binary), validates keys (`[a-z0-9_-]`), parents and cycles, and resolves each blueprint from its oldest ancestor down:

- `plugins.exclude` removes plugins inherited from ancestors; a blueprint's own `required` and `recommended` are applied after its `exclude`. A plugin that is required is never also recommended.
- `navigation.home` is overridden by the child; sections merge by `key` (the child replaces the label and appends items).
- `roles` merge by `key` (permissions are unioned).
- `configuration.packages` are configuration package template keys (`blueprints/<key>/config/<locale>.json`) applied once at install, parent first; `configuration.theme` is overridden.

## Installer

`aphel_kernel::application::installer::modules::profiles()` is fed by the blueprints (`blueprints/installer.rs`): each blueprint becomes a profile whose modules are its `required` and `recommended` plugins. `cargo run -p xtask -- modules sync` fails when an optional compiled module appears in no blueprint. `aphel.modules.toml` lists where each plugin comes from and its `[build] blueprints = [...]` chooses what is compiled (see below).

## Shipped blueprints

`crm` is today's product: the eight core plugins as `required` plus the shared recommended set. `freelancer`, `agency`, `real_estate`, `consultancy`, `saas`, `sales` (B2B) and `other` extend it with their own recommended plugins. `freelancer`, `agency`, `real_estate`, `saas` and `sales` also carry a configuration package per locale in `config/<locale>.json`; the package templates catalog reads them, with the blueprint key as the template key.

## Applied at install

- `plugins.required` plugins are enabled and locked by the installer.
- `[[roles]]` are created after the install (only permissions the build knows are granted, organization wide; existing roles with the same key are left alone).
- `navigation.sections` replace the module-declared sidebar sections: `items` are plugin keys; sections the blueprint does not claim keep their module-declared ones. A label is plain text or a table per language (`label = { en = "Pipeline", "pt-BR" = "Funil" }`).
- `navigation.home` is where `/` goes after sign-in when the user has no home of their own (a personal preference or a plugin role home win), when it is not `/`; if no module serves `/` the blank home redirects there too.
- `configuration.theme` is a path relative to `blueprints/` (for example `crm/themes/warm.json`) to an `aphel-theme` package; it is installed and activated through the `themes` module when that module is enabled.

## Install-time configuration

After the installation transaction the installer enables the blueprint's required plugins and applies its `configuration.packages` through the `config_packages` module (`BlueprintConfigurator`), with the administrator's scope. `crm` ships the default pipeline and lost reasons (localized in `config/<locale>.json`); a blank system, or a build without `config_packages`, applies nothing. If applying fails, the message is kept in `organization_settings.settings.installer.configuration_error`, `GET /api/v1/blueprint` returns it and administrators see a banner with a retry button (`POST /api/v1/blueprint/configuration`, `settings.manage`). Other templates in `blueprints/*/config/` stay selectable as packages and are suggested by onboarding.

Enabling a module later does not create defaults: they come from the blueprint package (reapply it with `POST /api/v1/blueprint`).

## Choosing what is compiled

`back/aphel.modules.toml` keeps one source entry per plugin (`path`, `git` or `version`) and a `[build]` table:

```toml
[build]
blueprints = ["freelancer"]
```

`make modules` (`cargo run -p xtask -- modules sync`) compiles the union of the resolved `required` and `recommended` plugins of the listed blueprints (the shipped file lists every blueprint, which is today's full build); without `[build]` every entry is compiled and each must appear in some blueprint. A listed plugin without a source entry fails the sync. The installer only offers blueprints whose plugins are all compiled, so a small build never shows a blueprint it cannot install. Plugin lists must be closed under dependencies: the kernel refuses to start otherwise, naming the missing module.
