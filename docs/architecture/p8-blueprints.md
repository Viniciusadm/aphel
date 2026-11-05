# P8 blueprints and blank core: manual acceptance

Compilation, lint, the arch check and `make probe` do not prove behaviour. Check by hand on a fresh database (collation `utf8mb4_unicode_ci`):

## Installer
- [ ] Every business type (freelancer, agency, real estate, consultancy, SaaS, B2B sales, other) installs; the features step shows the blueprint's required plugins as included and the rest preselected as before.
- [ ] "Start with a blank system" installs with only the kernel enabled; the home shows the blank page and the sidebar has only administration.
- [ ] After a blueprint install the default pipeline and lost reasons exist, in the installer language (`Funil padrão` in pt-BR).
- [ ] `GET /api/v1/blueprint` returns the chosen blueprint, or `null` for blank.

## Modules without `required`
- [ ] Disable `people`, `deals` or `notes` from Settings → Modules when nothing depends on them; the web and API keep working without them.
- [ ] Enabling a module later seeds its defaults on the next restart (`sync_defaults`), not immediately.

## Plugins
- [ ] A runtime plugin with `provides.contracts = ["<built-in contract>@1"]` holds the built-in module on install and releases it on uninstall; the plugin details show `Replaces modules`.
- [ ] A manifest with `conflicts_with` or `[[menu.rules]]` is rejected as an unknown field.

## Checked in Chrome (2026-11-06)
- Agency install: required plugins shown as always included, the rest preselected; dashboard home loads; `GET /api/v1/blueprint` returns `agency`; the default pipeline is `Funil padrão` with six localized stages and the Kanban shows them.
- `notes` and `people` disable when nothing depends on them (`/people` becomes a 404, the sidebar drops Contacts, deals keeps working); `deals` refuses with `module_required_by` while `deals_revenue` is on; both re-enable.
- Blank install (earlier session): only `kernel` enabled, blank home with module and settings links.
- Every other business type (freelancer, real estate, consultancy, SaaS, B2B sales, other) installed through the API on a fresh database with the profile's recommended modules (those without required settings): all return 201, the blueprint endpoint returns the chosen key, the blueprint's required plugins are enabled, `Funil padrão` and six lost reasons exist and the server log has no errors. The UI selection step was only exercised for Agency.
- Not checked: a plugin that provides a built-in contract, manifests with `conflicts_with`.

## Known gaps
- Blueprint navigation sections, roles and theme are parsed and resolved but only `navigation.home` is applied.
- Structural `is_system` seeds (task types, call outcomes, lead disqualification reasons, deal member roles) remain module seeds, run only for enabled modules.
- `aphel.modules.toml` still decides what is compiled; build selection by blueprint is deferred.
