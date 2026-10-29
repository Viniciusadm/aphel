# Aphel community catalog

This directory is the official catalog: a JSON index (`index.json`) with business templates and modules, the template packages (`packages/`) and the index specification (`schema/index.schema.json`). Anyone can publish their own catalog in the same format and point an installation to it.

## How Aphel uses the catalog

- By default the installation uses the **embedded catalog**: this `index.json` and the files in `packages/` are compiled into the binary, so the catalog works without internet access and follows the Aphel version. The screen shows the source `builtin:index.json`.
- `CATALOG_INDEX_URL=https://...` replaces the embedded catalog with a remote index; `CATALOG_INDEX_URL=builtin` goes back to the embedded one and `off` (or empty) disables the catalog.
- Each package's `url` can be absolute (HTTPS) or a path relative to the index (e.g. `packages/aphel.b2b-1.0.0.json`), resolved against the remote index URL or against the embedded catalog. Checksum, minimum version and `id`/`version` are checked the same way for both sources.
- Remote index: HTTPS only. To test a local index in development, use `CATALOG_ALLOW_HTTP=true` (never in production). Timeout configurable in `CATALOG_TIMEOUT_SECS` (default 10 s); the index can be up to 1 MiB and each package up to 2 MiB.
- Each entry is validated; invalid entries appear as "ignored" on the screen without bringing down the others.
- **Templates** (`kind: "template"`) are versioned `aphel-config` packages. Before installing, Aphel downloads the package, checks the **SHA-256 of the file's exact bytes**, the **minimum Aphel version** and whether the package's `id`/`version` match the entry; then it shows the comparison with the installed version (conflicts with local changes) and installs through the versioning flow (Settings > Configuration packages > Versions).
- **Modules** (`kind: "module"`) are **never downloaded or executed**. The screen only shows the description and the instructions: modules are Rust crates declared in `back/aphel.modules.toml` and compiled into the binary (see [`docs/modules/README.md`](../docs/modules/README.md)).

## Index format

```json
{
  "$schema": "./schema/index.schema.json",
  "format": "aphel-catalog",
  "format_version": 1,
  "name": "My catalog",
  "entries": [
    {
      "id": "acme.clinicas",
      "kind": "template",
      "name": "Clínicas",
      "description": "Funil de agendamento e retorno de pacientes.",
      "author": "ACME Consultoria",
      "version": "1.2.0",
      "min_aphel_version": "0.1.0",
      "license": "CC-BY-4.0",
      "documentation_url": "https://exemplo.org/clinicas",
      "tags": ["saúde"],
      "locale": "pt-BR",
      "package": {
        "url": "https://exemplo.org/pacotes/acme.clinicas-1.2.0.json",
        "sha256": "<64 lowercase hexadecimal characters>",
        "size": 18342
      }
    },
    {
      "id": "acme.module.sms",
      "kind": "module",
      "name": "SMS",
      "author": "ACME",
      "version": "0.3.0",
      "min_aphel_version": "0.1.0",
      "license": "AGPL-3.0-or-later",
      "documentation_url": "https://exemplo.org/sms",
      "module": {
        "key": "sms",
        "repository": "https://exemplo.org/aphel-sms",
        "install_notes": "Declare sms = { git = \"https://exemplo.org/aphel-sms\", tag = \"v0.3.0\" } in back/aphel.modules.toml, run make modules and compile."
      }
    }
  ]
}
```

Required fields for every entry: `id` (stable, lowercase letters, digits and `.`/`-`/`_`), `kind`, `name`, `author`, `version` (SemVer), `min_aphel_version` (`X.Y.Z`) and `license` (SPDX). Templates require `package` (`url` HTTPS or relative to the index, and `sha256`); modules require `module` (`key`) and cannot have `package`. The full specification is in [`schema/index.schema.json`](schema/index.schema.json).

## Publishing a template

1. Export the configuration in Settings > Configuration packages > Export and review the JSON (no customer data: packages only carry configuration).
2. Add `"id"` and `"version"` to the manifest. The `id` never changes between versions; the `version` follows SemVer (a change that alters existing items: minor; only new items: patch; key changes: major).
3. Save the file as `packages/<id>-<version>.json` and add the entry to `index.json` (without `package`).
4. Run `node catalog/scripts/build.mjs`: it computes each template's `sha256`, `size` and `url` (relative, `packages/<file>`) and regenerates the packages of the built-in templates (`aphel.*`) from `back/config-packages/templates`. `node catalog/scripts/build.mjs --check` verifies without writing. Since the catalog is embedded, the change only reaches installations in the next Aphel version (or through a remote index).
5. Open a pull request. The contribution guide is in [`docs/community/catalog.md`](../docs/community/catalog.md).

A published version is never rewritten: fixing a package means publishing a new version. Changing the file without changing the version breaks the checksum and Aphel refuses the installation.

## Your own catalog

Serve an `index.json` and the packages over HTTPS (GitHub Pages, a bucket or any static server) and set `CATALOG_INDEX_URL`. With the packages in `packages/` next to the index, the relative paths generated by the script already work. To generate absolute URLs for another address: `node catalog/scripts/build.mjs --base-url https://exemplo.org/pacotes --output /path/index.json` (and `--check --base-url` with the same address to verify).
