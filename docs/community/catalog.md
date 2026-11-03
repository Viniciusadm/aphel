# Contributing to the catalog

The official catalog lives in [`catalog/`](../../catalog) in this repository and is embedded in the binary: what goes in here reaches installations in the next Aphel release, without depending on internet access (Settings → Integrations → Modules → Catalog switches to a remote index). The index specification and the publishing walkthrough are in [`catalog/README.md`](../../catalog/README.md); this guide covers what the review expects.

## Business templates

A template is an `aphel-config` package with an `id` and a `version`. Before opening the pull request:

- **Scope:** configuration for a type of business (fields, workflow, loss reasons, activity types, tags, layouts, views, dashboard, automations, custom objects). No customer records, users, emails or secrets: the format does not even accept them, and the review rejects real data in descriptions and examples.
- **Identity:** `id` in the `<author>.<template>` format (e.g. `acme.clinicas`). The `aphel.` prefix is reserved for templates maintained by the project.
- **Stable keys:** items are matched by `key` across versions. Renaming a key is equivalent to removing the old item and creating another; avoid it, and when it is unavoidable publish it as a major version and explain it in the description.
- **Versions:** SemVer. A published file never changes; a fix is a new version. `min_aphel_version` must be the lowest version in which the package passes validation (`POST /api/v1/config-package-versions/compare` returns `valid: true`).
- **Language:** the package's `locale` must match its texts. To offer the same template in two languages, publish two `id`s (e.g. `acme.clinicas` and `acme.clinicas-en`).
- **License:** declare an SPDX license that allows redistribution (recommended: `CC-BY-4.0` or the project's, `AGPL-3.0-or-later`).
- **Documentation:** `documentation_url` (optional, HTTPS) explaining who the template is for, the proposed pipeline and the meaning of each field.
- **Technical checklist:** package validated on a clean installation and upgraded from the previous version (when there is one), `node catalog/scripts/build.mjs --check` without errors and an entry valid against `catalog/schema/index.schema.json`.

## Modules

The catalog only **lists** modules. As a principle (session 19 of the plan), Aphel never downloads or runs third-party code: a plugin is a crate (in `back/plugins/<key>` or in a git repository) that goes into the binary through a line in `back/aphel.modules.toml` and `make build` (see [Modules and plugins](../modules/README.md)).

- Modules maintained outside the repository enter the catalog with `kind: "module"`, `module.repository` pointing to the code and `install_notes` with the steps (declare it in `back/aphel.modules.toml`, run `make modules`, compile, enable it in Settings > Integrations > Modules).
- The review checks for a compatible license, a manifest with `min_aphel_version`, tables prefixed with `<key>_` and no shortcuts into the `aphel` crate (the full guide is in [`docs/modules/README.md`](../modules/README.md)).

## Review and removal

Maintainers review every new entry or new version. An entry can be marked as removed (taken out of the index) if the package becomes inaccessible, violates its license or contains personal data; installations that already applied the template keep working, because installing never creates a runtime dependency on the catalog.
