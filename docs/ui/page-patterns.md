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
| Fixed options, including empty options | `NativeSelect` or `SelectField` |
| Shared form dialog structure | `DialogForm` inside `DialogContent` |
| Client-side search, filters and pagination | `CollectionView` |
| Standalone public or system title | `PageTitle` |
| Dialog widths | `DialogContent size="sm|md|lg|xl|2xl|3xl"` |

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

## Collections

Choose `Table` for comparable columns and `ItemList`/`ItemRow` for variable row content with actions. Use `CollectionView` for a complete in-memory collection: it composes `SearchInput`, `NativeSelect`, empty feedback and `Pagination`. Pass translated labels and domain-specific search/filter accessors. It never fetches or changes data. Server-paginated collections must keep their server pagination rather than paginate a partial response again.

## Dialog forms

Use `DialogForm` inside `DialogContent` for forms with a standard cancel/submit footer. Pass translated title, optional description, labels, submission error and pending state. Keep validation and field composition in the caller. Specialized dialogs may compose the same header and footer primitives directly.

## Titles and settings

`PageHeader` uses `PageTitle` for page-level headings. Standalone public, authentication and system layouts use the same title primitive. Settings area overviews own their page frame; internal routes render their own frame without a second wrapper from the area layout.

Use `size="3xl"` for a dialog editor with a separate live preview. Ordinary forms retain the smaller standard sizes. Rich record choices with colored option labels retain Radix `Select`; all plain text choices use native selection, including layout tools, theme previews and phone country selection.

Sidebar navigation uses `SidebarLink`, including settings navigation, collapsed icon links and nested group entries. Keep tooltips and route permissions in the application shell. Group controls retain their native button semantics through `asChild`.

Dialogs with no description set `aria-describedby={undefined}` on `DialogContent`. Complex dialogs with an external form footer, an upload flow or extra actions may compose `DialogHeader`, `Field` and `DialogFooter` directly instead of adding variants to `DialogForm`.
