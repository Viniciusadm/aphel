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

## Checked in Chrome after the follow-up (2026-11-06)
- A blueprint whose package does not exist: the install succeeds, `GET /api/v1/blueprint` returns `configuration_error`, admins see the banner, and after fixing the package the retry button applied it (pipeline created, banner gone).
- A blueprint with `[[roles]]` creates the role with only the valid permissions; a blueprint with `navigation.sections` shows its own sidebar section (`Pipeline` with Empresas and Oportunidades). These two used a temporary change to `blueprints/crm`, reverted afterwards.

## Checked at the end (2026-11-06)
- Blueprint `theme`: installing with a temporary theme in `blueprints/crm` made `/api/v1/public/themes/active.css` return the theme tokens.
- Blueprint `home = "/tasks"` redirected `/` to `/tasks` after sign-in; a localized section label (`Funil`) showed in pt-BR. Both used a temporary `blueprints/crm`, reverted.
- Plugin contract hold with a manifest-only package providing `booking@1`: dry run listed `replaces: ["booking"]`, the install disabled `booking` (`held_by` shown), enabling it was refused with `module_held_by_extension`, and uninstalling re-enabled it. This found and fixed a foreign key failure (the hold ran before the plugin row existed).
- A manifest with `conflicts_with` or `[[menu.rules]]` is rejected with a clear message.

## Known gaps
- Structural `is_system` seeds (task types, call outcomes, lead disqualification reasons, deal member roles) remain module seeds, run only for enabled modules (decision: hybrid).
- `aphel.modules.toml` still decides what is compiled; build selection by blueprint is deferred (decision).
