# Page patterns

All screens, in the core and in plugins, are built from the same primitives exported by `@aphel/ui`.

| Need | Use |
| --- | --- |
| Page container | `Page` (`width`: `full`, `form`, `narrow`) |
| Title, description, badges, actions, back link | `PageHeader` (`level="section"` for a sub-section of a page) |
| Section inside a page | `SectionTitle` |
| Loading, error and empty states of a query | `QueryState` |
| Tabular data | `Table` family (bordered container, shared paddings) |
| Pagination of non-record lists | `Pagination` |
| Tabs stored in the URL | `UrlTabs` |
| Master list with a detail pane | `MasterDetail` |
| Form fields | `Field` |
| Select with an empty option | `SelectField` |
| Dialog widths | `DialogContent size="sm|md|lg|xl|2xl"` |

Rules:

- Every page renders exactly one `PageHeader` (`h1`, `text-xl`) as the first child of `Page`.
- Detail and editor pages pass `back` to `PageHeader`; they never build their own back link.
- Primary actions go in `PageHeader actions`; creation forms open in a `Dialog`.
- Plugins never reimplement these components. A missing pattern is added to the core and exported in `web/src/sdk/ui.ts`.
- A plugin groups its sidebar entries with the `group` field of `app.nav`.
