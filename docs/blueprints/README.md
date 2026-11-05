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

`aphel_kernel::application::installer::modules::profiles()` is fed by the blueprints (`blueprints/installer.rs`): each blueprint becomes a profile whose modules are its `required` and `recommended` plugins. `cargo run -p xtask -- modules sync` fails when an optional compiled module appears in no blueprint. `aphel.modules.toml` still decides what is compiled.

## Shipped blueprints

`crm` is today's product: the eight core plugins as `required` plus the shared recommended set. `freelancer`, `agency`, `real_estate`, `consultancy`, `saas`, `sales` (B2B) and `other` extend it with their own recommended plugins. `freelancer`, `agency`, `real_estate`, `saas` and `sales` also carry a configuration package per locale in `config/<locale>.json`; the package templates catalog reads them, with the blueprint key as the template key.

## Install-time configuration

After the installation transaction the installer enables the blueprint's required plugins and applies its `configuration.packages` through the `config_packages` module (`BlueprintConfigurator`), with the administrator's scope. `crm` ships the default pipeline and lost reasons (localized in `config/<locale>.json`); a blank system, or a build without `config_packages`, applies nothing. Other templates in `blueprints/*/config/` stay selectable as packages and are suggested by onboarding.
