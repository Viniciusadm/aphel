# P7 web parity: manual acceptance

Typecheck, oxlint and the arch check prove that the code compiles and the counts hold. They do not prove behaviour. Check by hand:

## SDK imports (P7.1)
- [ ] Open a record list, form, detail page and the settings screens: nothing renders blank after the move to `@/sdk` (watch the browser console for circular import errors on first load).
- [ ] Plugin settings panel (`wasm_runtime`) still opens.

## Slots (P7.2)
- [ ] Reports library shows the deals cards; dashboard top, dashboard sources and the themes section of the catalog still render.

## Settings areas and nav (P7.3)
- [ ] Account, organization, integrations and data areas appear with titles, search and "other areas" links.
- [ ] The `sales` group appears between records and communication in the organization area, with its translated label, in pt-BR and en.
- [ ] Disable the modules that use `sales`: the group disappears.

## Locales (P7.4)
- [x] Language menu in the user menu and in the navigation settings lists pt-BR and en with the same names as before; switching still works and persists.
- [ ] Add a module with a third locale: it appears in both menus (preference saving is limited by the backend's `pt-BR`/`en` enum).

## Contract components (P7.5)
- [ ] Deals reports show the period selector and compare toggle with the dashboard enabled; with the dashboard disabled the controls are absent and nothing crashes.
- [x] The rotting badge shows in the deals list with workflow enabled. Workflow cannot be disabled while `deals`, `quotes` or `renewals` are enabled (they now declare the dependency), so "absent without it" no longer applies.
- [ ] Deal links in renewals, forecast, leads conversion and deal revenue open the deal; with deals disabled the link is inert.

## Record types, sidebar, tabs (P7.6 to P7.8)
- [ ] Field, field permission, layout and validation rule settings list the record types from the entity catalog, redirect to the first one and show the right labels.
- [ ] Sidebar sections (work, records, operations, analysis, administration) hold the same items as before; custom objects sit in the records section; the settings entry is last.
- [ ] Mobile record overview groups tabs (activity, relationships, communication, insights, more) as before for contact, company, deal, task.
- [ ] Record summary panel shows the expected fields for deals and contacts, and every field for a resource without `summaryFields`.

## Per-module API types (P7.9)
- [ ] `make api-types` regenerates the 24 `modules/*/api-types.d.ts` files without a diff on an unchanged OpenAPI document; `/api/openapi.json` shows `x-aphel-module` on operations and tags.

## Trait-aware UI (P7.10)
- [ ] The related deals tab appears on contacts and companies only if the entity catalog reports `party.contact_points` in `traits` (not verified against the backend response).
- [x] A custom object with `staged: true` (Settings → Custom objects → Edit → "Records move through stages") shows a Stages view next to List: columns are the pipeline stages, the pipeline picker appears with more than one pipeline, stage counts and the "stage has records" block on delete include its records. Verified in Chrome; dragging a card was not exercised (the board component is the generic one).
- [ ] The Files tab still appears for every entity (attachments are a kernel capability for any readable record, so no trait gates it); the deals page keeps its own board.

## Build without plugins
- [ ] With `aphel.modules.toml` empty, `make modules` and `npm run typecheck` pass (checked once; restore the generated files afterwards).

## Shell words to zero (P7.6 to P7.8 follow-up)
- [ ] `G` + c/e/d/p/t/i/a/r shortcuts work and show in the shortcuts help only for enabled modules.
- [x] Mobile bottom bar shows the pinned entry (deals) only when that module is enabled.
- [ ] Audit filters and rows, and command palette result groups, show entity labels (plural) from the catalog.
- [ ] Installer first step is labelled as before (key renamed from company to organization) in pt-BR and en; roles matrix shows the module labels and the assignment permission label.
- [x] A stage checklist error and a validation rule error still show their messages.
