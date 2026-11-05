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
packages = ["config/en.json"]
theme = "themes/default.json"
```

## Resolution (`extends`)

The `aphel-blueprint` crate (`back/blueprint`) parses every `blueprints/*/blueprint.toml` (embedded in the binary), validates keys (`[a-z0-9_-]`), parents and cycles, and resolves each blueprint from its oldest ancestor down:

- `plugins.exclude` removes plugins inherited from ancestors; a blueprint's own `required` and `recommended` are applied after its `exclude`. A plugin that is required is never also recommended.
- `navigation.home` is overridden by the child; sections merge by `key` (the child replaces the label and appends items).
- `roles` merge by `key` (permissions are unioned).
- `configuration.packages` accumulate parent first; `configuration.theme` is overridden.

## Installer

`aphel_kernel::application::installer::modules::profiles()` is fed by the blueprints (`blueprints/installer.rs`): each blueprint becomes a profile whose modules are its `required` and `recommended` plugins. `cargo run -p xtask -- modules sync` fails when an optional compiled module appears in no blueprint. `aphel.modules.toml` still decides what is compiled.
