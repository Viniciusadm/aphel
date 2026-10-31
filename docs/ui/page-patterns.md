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

## Select controls

Use `NativeSelect` for fixed options in forms and list filters. Use `SelectField` when a field needs a placeholder or empty option. Both preserve native mobile selection. Reserve Radix `Select` for rich option content or grouped options requiring custom rendering.

## Form feedback

Pass validation messages through `Field error` and descriptions through `Field description`. Use the child callback to associate controls with their label and feedback. Display submission errors with `Alert`.

Form dialogs use `DialogHeader`, a body composed of `Field`, and `DialogFooter` with cancel and submit actions. Disable submission while pending and preserve values after failure.
