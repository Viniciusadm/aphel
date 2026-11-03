# P5.12 Entity reports and owner contributions

Reports owns saved aggregate definitions, access to those definitions, execution/export and delivery schedules. Its entity catalog comes from readable registry definitions and enabled owners. The report builder no longer fixes a list of CRM entities or permissions.

## Static review

- [x] Conversion, sales performance, their DTOs, SQL and presentation belong to deals. They are served through `/deals/reports/conversion` and `/deals/reports/performance`; no compatibility routes remain.
- [x] Reports has no deals, workflow, dashboard or transitional CRM implementation dependency. Period data and transport schemas and dashboard widget ports are shared through the SDK.
- [x] The report library renders contributed owner cards through an open slot. Deals contributes its own routes, permission requirements, localized descriptions and report UI.
- [x] Saved report entity options come from `/reports/entities`, with declared read permissions, owner availability and localized entity labels. Native, managed and dynamic declarations use the same path.
- [x] Saved definitions still run through the aggregate engine with the caller's scope and field permissions. Export and scheduled delivery preserve the existing scoped execution.
- [x] Reports queries its own saved definitions, recipients and deliveries and core identity/settings data. Its domain-specific historical calculations no longer live in the host.
- [x] Dashboard remains optional: a saved-report widget contributes through the SDK and does not require the dashboard implementation.

## Manual acceptance pending

No automated tests, probes or runtime instances were executed under D4. Verification is limited to compilation, strict lint, type checks, frontend build and route/catalog generation.

- [ ] Create and run saved reports over native, managed and dynamic entities with limited record scopes, hidden fields, relation groups and previous-period comparisons.
- [ ] Inspect library cards with deals disabled and with/without deals.read, including direct owner report URLs and both locales.
- [ ] Verify conversion/performance calculations, pipeline visibility, monetary field masking and task scopes through the owner routes.
- [ ] Inspect entity removal/disablement, empty catalogs, deleted report definitions, exports and scheduled recipients with revoked permissions.
- [ ] Install reports without dashboard and inspect saved reports and schedules; enable dashboard and inspect the contributed saved-report widget.

Deals still needs its workflow-owned lookup and stage-history query boundaries reviewed in P5.19. Reports retains export and saved-view implementation helpers for the shared-capability cleanup in P5.22–P5.24. Moving report ownership does not claim those remaining dependencies are resolved.
