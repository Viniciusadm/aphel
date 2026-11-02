# Module compatibility

Since session 36 there is no longer a separate extension API (`aphel-extension-api` was removed in session 35): a module or plugin compiles against Aphel's own crates — `aphel-kernel` and the `aphel-mod-<key>` crates it uses. The contract is the public API of those crates plus the rules below.

## Versions

| Version | Where | What it is for |
| --- | --- | --- |
| Aphel | `[workspace.package] version` in `back/Cargo.toml`; it is the version of `aphel-kernel`, of the binary (`APHEL_VERSION`) and of every module that ships with Aphel | Modules' `requires_aphel`, `aphel doctor`, `aphel upgrade`, status screen |
| Module | `Manifest::new(key, name, version)`; Aphel's modules use `APHEL_VERSION`, a plugin uses its own (`env!("CARGO_PKG_VERSION")`) | Other modules' `depends_on(key, requirement)` and the installation's `installed_version` |

All of them follow semver. While Aphel is at `0.x`, a minor version (`0.1` → `0.2`) may bring a breaking change; from `1.0` on, only a major version.

## How Aphel decides whether a module runs

1. Two modules with the same key: the second one is ignored (error in the log). A circular dependency between manifests prevents the binary from starting.
2. `requires_aphel(min)` greater than the binary's version: the module is listed as incompatible (`requires_aphel`) and registers nothing (routes, hooks, services, worker, migrations).
3. Each `depends_on(key, requirement)` needs a compiled module (`missing_dependency`) whose version matches the requirement (`incompatible_dependency`); the incompatibility propagates to its dependents. `optional(key)` only orders loading.
4. `installed_version` (`installed_modules` table) stores the enabled version; when the binary ships a different one, enabling it again records the new one. New migrations run on `aphel migrate` and at startup.

`aphel modules list` shows the compatibility of each module in the build.

## What is stable

- `Module`, `Manifest` and `AppBuilder` (`aphel_kernel::modules`): new methods come with a default implementation or as new builder methods; they do not change signature within a compatible version.
- Declared hooks (catalog in [hooks.md](hooks.md)): the hook type, the argument type and when it runs. New fields may appear in the arguments (build and read them by field, not by full destructuring).
- Services (`provide`) and typed contributions listed in the [guide](README.md): the trait may gain methods with defaults.
- Naming rules: module key, `<key>.` permissions, `/api/v1/<key>` routes (public ones at `/api/v1/public/<key>`; a key with `_` becomes `-` in the path), web `/<key>`, i18n namespace `<key>`, `<key>_*` tables, `<key>.` jobs.
- UI library: the exports of `@aphel/ui` and the design token names ([docs/ui](../ui/README.md)) only grow; default token values may be refined. The `aphel-theme` package format ([docs/themes](../themes/README.md)) only grows within version 1.
- Web: `defineModule`/`ModuleApp`, slots, filters, override ids and field type lookup described in [web.md](web.md); new contributions are additive.
- Public HTTP API (`/api/v1`): see [API versioning](../api/versioning.md).

Any other `pub` item in the crates (stores, internal DTOs, a module's service functions) may be used, but may change in any version: depend on it knowing that, and pin the version (`tag`) of Aphel and of the plugins.

## What counts as a breaking change

Removing or renaming a stable item, changing a signature, changing when or in which transaction a hook runs, changing a hook's argument type, changing a naming rule or the semantics of scope, permissions and events, or changing a table's schema in a way that breaks an FK or a column added by a plugin. This is announced in the release notes.

## Migrations

- Numbered within the module, applied in order and recorded in `module_migrations (module_key, version, checksum)`; modules run in load order (dependencies first).
- An applied migration never changes; fix it with a new one.
- A plugin may alter other modules' tables (`ALTER TABLE contacts ADD COLUMN ..`) as long as it declares `depends_on` on the owner and reverts the change in `uninstall_sql`.
- `aphel modules uninstall <key> --purge` runs `uninstall_sql` and deletes the module's migration records, which run again if it is reinstalled.

## Third-party plugins

A plugin is a crate outside the repository (or in `back/plugins/<key>`) referenced in `back/aphel.modules.toml` by `path`, `git` (with `tag`, `branch` or `rev`) or `version`. When upgrading Aphel, check the release notes for changes in the kernel and in the modules the plugin uses, and update the plugin's `tag` to a compatible one.

## History

- **End of extension names:** the `/api/v1/ext/<key>` routes became `/api/v1/<key>` and the public `/api/v1/ext/<key>/public/…` ones became `/api/v1/public/<key>/…`; everything under `/api/v1/public/` is exempt from CSRF (the `CsrfExemptPrefix` contribution was removed); web `/x/<key>` became `/<key>`; `ext_<key>_*` tables became `<key>_*` (the legacy prefix was removed from the purge); `ext.<key>.*` jobs became `<key>.*`; `extension_*` codes became `module_*` (`module.*` events, module secrets with `owner_type = "module"`); the catalog uses `kind: "module"`.
- **Session 36:** modules become crates (`aphel-kernel`, `aphel-mod-<key>`, `aphel` binary in `back/app`), selection via `aphel.modules.toml`, local plugins in `back/plugins`. New extension points: `Manifest::uninstall_sql`, `AppBuilder::declare_hook` and `require`, the `ConfigSectionFactory`, `CsrfExemptPrefix`, `DefaultsSeed`, `TimeTriggerPlanner`, `ModuleCommand` and `TimelineEventExtension` contributions, the `SearchPlanFilter` hook and automation operation services. The automation gateway lost `distribute`, `send_owner_email`, `enroll_in_sequence` and `request_approval` (each module calls its own service through `gateway.state()`).
- **Session 35:** end of `aphel-extension-api` (last version `2.0.0`); the former extensions become regular modules.
