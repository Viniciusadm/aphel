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
| Contextual sections stored in the URL | `SectionWorkspace`, `SectionIndex`, `SectionLink`, `SectionContent` |
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
- Primary actions go in `PageHeader actions`. Use dialogs for short creation flows and dedicated pages for full record editing.
- Plugins never reimplement these components. A missing pattern is added to the core and exported in `web/src/sdk/ui.ts`.
- A plugin groups its sidebar entries with the `group` field of `app.nav`.

## Information architecture

Use the application sidebar for product destinations, `AreaNavigation` for closely related route views, and `SectionWorkspace` for independent functions within a complex area. Contextual section selection uses the URL and preserves browser navigation. Do not use horizontal tabs as primary page navigation. Retain tabs only for equivalent modes within a single task, such as writing and previewing a message. Shared TabsList and SegmentedControl render native selection controls below 768 pixels. Use a labeled filter dialog with a vertical checkbox list for multi-select timeline event types; never represent those filters as a horizontally scrolling navigation strip.

Record pages open an overview with upcoming work and recent activity. Full collections and complete fields have contextual destinations. Desktop uses a main work area and a compact supporting field panel; mobile presents the next actions before supporting fields and recent activity. Configure domain priorities with `RecordResource.summaryFields`; saved layout visibility rules still apply.

Full editing opens `/records/:recordType/:recordId/edit`, preserves the previous record location and guards unsaved changes. Short creation and contextual actions continue to use dialogs. Form layout groups render as vertical chapters with section links; their persisted schema is unchanged.

`RecordTable` uses cards on mobile, including selection, multi-field sorting, row actions and disclosure of additional selected columns. Keep server queries and pagination shared across presentations. Mobile workflow show one selected stage and explicit movement actions.

Use `app.recordSection` for module contributions. It supports an optional group and summary renderer. `recordTab`, existing record filters, record page options and `?tab=` URLs remain compatibility interfaces. The `UrlTabs` export adapts its existing contract to contextual sections.

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

## Mobile collection controls

Keep the saved view above a single search-and-filter row. Put creation in the page header. On record lists, the filter button opens a bottom sheet with criteria, archive visibility, optional sorting and display sections. Keep a short applied-filter/sort summary outside; do not stack every control or duplicate chips and sorting panels above the cards. Apply all draft options in one URL update and reset pagination; dismissing the panel discards its draft. FilterMenu also opens a bottom sheet on mobile for consumers outside RecordListView.

## Mobile page actions and pagination

Below 768 pixels, page-header actions follow the title and description on their own row; a single action can use the available width. Desktop retains the side-by-side header. Global quick entries belong in the header create menu, not in a floating button over list content. Preserve the current record when opening a quick-entry form. Pagination contains only Previous, Page X of Y and Next on both mobile and desktop; render PaginationSummary beside the result list and place PageSizeSelect inside Display settings; reserve bottom spacing for the fixed navigation and safe area only.

The Pagination API takes page, perPage, total and onPageChange. Move former onPerPageChange/perPageOptions usage to PageSizeSelect and reset to page 1 when changing size. PaginationSummary is a separate component so counts never become an interactive page indicator. These primitives are exported through @aphel/ui.


## Operational workspaces

Use `WorkspaceOptions` for infrequent scope and display controls. It uses a bottom sheet on mobile and a side panel on desktop; pass a translated label and an optional active count. Changes are immediate, so transactional filter builders keep their own Apply/Cancel flow.

Use `record.list.actions` for record collection operations such as import and export. Keep query controls in `record.list.toolbar`; do not move existing plugin contributions between slots implicitly.

Full creation uses `/records/:recordType/new`; contextual creation can transfer its draft there. Declare additional collection query keys on a record resource when creation changes specialized collections. Keep long editors on routes and reserve dialogs for bounded actions.
