# P5.13 Contributed dashboard data and presentation

Dashboard owns widget discovery, scoped execution, layouts, subject selection and generic source collections. Providers declare their default placement and reference settings through SDK metadata. Business calculations and presentation belong to their native owners.

## Static review

- [x] Deals owns sales indicators, stage counts, stale-work queries, monetary field masking and their web renderers. Pipeline selection applies workflow visibility.
- [x] Tasks owns pending task queries, date buckets and field masking. People, tasks and deals contribute their own summary counts and period comparisons.
- [x] Pending and summary widgets merge bounded, validated, namespaced SDK source sections. The generic web host renders owner-contributed sections without interpreting their business fields.
- [x] Default layouts use provider order/size/settings metadata instead of a fixed CRM widget list. Widget availability and selected subjects use owner/entity permissions.
- [x] Recent history derives readable entity plans from the registry. Notes contributes its own private-note predicate and parent resolution. Imports contributes its own event exclusion through the SDK.
- [x] Recent event payloads use record field policies. A shared SDK record-label reader checks owner availability, record access and title visibility using the generic record engine; task links and recent events both consume it.
- [x] Event action labels and stage-change presentation are contributed by owners. The dashboard web host has no fixed business-event list or sales-specific renderer.
- [x] Native widget contexts receive the database pool explicitly. Extracted providers do not acquire the whole global application state.
- [x] Widget reference settings declare their reference kind and portable key field. Workflow owns pipeline key discovery; saved reports declares its local-only report selection. Dashboard package import/export/validation no longer imports workflow implementations or assumes pipeline/report fields globally.
- [x] Dashboard has no transitional CRM contract or workflow implementation dependency. Its remaining SQL reads its layouts and core event, identity and settings data; scoped entity access uses the generic query/record engine.
- [x] No compatibility routes, aliases or migration/adoption paths were introduced.

## Manual acceptance pending

D4 prohibits automated tests, probes and runtime instances. Validation uses compilation, strict Clippy/lint, type checks, frontend build and route/catalog generation without starting a server or connecting to the database.

- [ ] Inspect empty installations, default ordering, independently disabled owners and contributed widgets/sections with no accessible data.
- [ ] Compare sales totals, pipeline visibility, forecasts, pending task dates and stale work across own/team/organization scopes, timezones and previous periods.
- [ ] Verify hidden monetary, stage, task title/due-date/priority, owner and related-record fields; inspect links whose parent is private, deleted, archived or disabled.
- [ ] Inspect mixed recent events, private notes, imported-event exclusion, payload masking, unknown event actions and both locales.
- [ ] Export/import layouts with portable reference keys, package-created references, missing catalogs, unavailable widgets and non-portable saved-report selections.
- [ ] Inspect per-user/default layout editing, settings validation, user/team switching and a runtime plugin widget.

Deals still has workflow-owned SQL/lookups to dissolve in P5.19. Other widget contributors still import dashboard implementation aliases; those capability boundaries and remaining configuration-package dependencies remain in P5.22–P5.24. This item does not claim the final zero-dependency gate is complete.
