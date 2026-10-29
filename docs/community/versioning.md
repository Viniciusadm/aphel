# Configuration versioning

An `aphel-config` package (see [`docs/config-packages/README.md`](../config-packages/README.md)) gets an identity and a version when the manifest has two optional fields:

```json
{ "format": "aphel-config", "format_version": 1, "min_aphel_version": "0.1.0",
  "id": "aphel.b2b", "version": "1.1.0", "name": "Vendas B2B", "locale": "pt-BR", "sections": { } }
```

Packages without `id`/`version` keep working in the **Install** tab as before. Versioned packages are installed and updated through the **Versions** tab (Data > Configuration packages) or through the **Catalog** (`/data/catalog`).

## What gets recorded

Each versioned installation writes, in the same transaction as the application:

- a row in `config_package_installs` (the usual history, with the original package);
- a row in `config_package_versions`: package, version, source (catalog URL, when there is one), SHA-256, who installed it, when, and the **package actually applied** (with the "keep local" choices already merged);
- one row per item in `config_package_version_items`: the item's hash in the package and the resolution (`applied`, `unchanged`, `kept_local`, `skipped`);
- the `config_package.version_applied` event and the audit entry with one change per applied or kept item.

## Comparison

`POST /api/v1/config-package-versions/compare` (or `POST /api/v1/catalog/templates/{id}/compare`) compares the new package with the last installed version of the same `id`. Each item (field, pipeline, stage, loss reason, activity type, tag, layout, view, dashboard, automation, object, relationship) gets:

| Field | Meaning |
| --- | --- |
| `package_status` | `added`, `changed`, `removed` or `unchanged` between the installed version and the new one (hash of the item in the package) |
| `package_changes` | Properties that changed in the package (before → after) |
| `locally_modified` | The item was changed in the organization after installation: the plan of the applied package against the current configuration is no longer `unchanged`, or the item was kept local in the previous update |
| `conflict` | The package changed the item **and** it was changed locally |
| `plan` | The usual installation plan (create/update/unchanged/skip_conflict) against the current configuration |
| `needs_resolution` | Items with a version conflict or with a resolvable plan conflict |

Items removed in the new package show up as `removed`, but **are never deleted**: installing deletes nothing and does not touch business records. Module sections are compared as a single item per section.

## Applying and conflicts

`POST /api/v1/config-package-versions/apply` receives the package, the `fingerprint` from the comparison and `resolutions` per item: `keep_local` (default) or `accept_package`.

- **Keep local** in a version conflict replaces the package's item with the organization's exported version (or removes it, if it was deleted locally). The merged package is validated again and applied by the same `ConfigSection` (`plan`/`apply`) as the normal flow.
- **Accept package** applies the new version of the item.
- In resolvable plan conflicts (e.g. field type), `accept_package` becomes the `apply` decision and `keep_local` becomes `skip`.
- If the organization or the installed version changes between the comparison and the application, the response is 409 `plan_outdated` and the screen compares again.
