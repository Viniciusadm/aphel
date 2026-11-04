# Experience migration

## Decisions

The default audience is a general sales team. Navigation prioritizes daily work, customer context and next actions. Preserve Aphel branding, enabled-module boundaries, record permissions, saved views, custom fields, layout rules and existing REST contracts.

Desktop navigation groups work, CRM, operations, insights and administration. Sidebar search includes contributed group labels. Personal ordering remains effective within the new categories. Mobile exposes Today, Deals when permitted, Search and Menu; quick entry and account controls live in the mobile Menu alongside the standard creation actions. No floating creation button overlays list content or pagination. Page actions share the title area when space permits; record editing and secondary actions use labeled icon buttons. Pagination contains only Previous, Page X of Y and Next. Result ranges are rendered beside the list, and page-size settings belong in Display. Space is reserved only for the bottom navigation.

## Coverage inventory

- Shell, dashboard, search and following: shared page hierarchy, searchable grouped navigation and mobile destinations.
- Contacts, organizations, leads, deals, tasks, products and custom objects: shared record overview, domain field priorities, full detail destination, contextual collections and dedicated editing.
- Record collections, segments and relation lists using RecordListView or RecordTable: mobile cards, sorting, selection, row actions and saved-view behavior. Specialized comparison tables retain contained horizontal scrolling.
- Deals: desktop board and a mobile stage overview with totals, grouped records and an optional horizontal board, including explicit stage and pipeline movement, validation and existing approval flows.
- Tasks: related route views through AreaNavigation. Calendar keeps its purpose-specific representations and existing mobile agenda behavior.
- Messaging: existing desktop list/conversation and mobile sequential conversation flow retained. Shared shell, controls and feedback apply.
- Booking: bookings are the default destination; availability and meeting types have independent nested routes. Legacy tab query links redirect while preserving other parameters.
- Calls: sessions remain operational; reporting is independently discoverable under Insights; outcomes belong to organization settings. The previous outcomes URL redirects.
- Reports: library entry, permission-filtered built-in destinations, saved reports and dedicated report builder.
- Automations: flow, simulation, execution history and versions are contextual destinations; draft state and unsaved changes protection remain in the parent editor.
- Email: mailbox management, templates, organization blocklist and preferences use contextual sections. Write/preview modes remain local tabs.
- Approvals: native queue and status selectors replace navigation-shaped filters.
- Price lists, privacy, configuration packages and theme editing: independent functions use contextual workspaces. Theme preview retains a small mode selector.
- Sequences, proposals, quotes, lead forms, imports, exports, bulk history, duplicates and merge flows: existing collection/editor or sequential workflows retained; shared navigation, record contributions and controls are migrated.
- Goals, forecast, scoring, distribution, currencies, workflow, tags and specialized modules: shared shell and controls apply; contributed record content uses the section API.
- Organization, users, teams, roles, fields, object definitions, layouts, validation, integrations, webhooks, storage, modules, catalog, notifications, account security, audit, trash and system: existing searchable settings architecture and dedicated pages retained. Multi-function configuration pages use contextual sections.
- Authentication, installation, public booking and public quotes: standalone flows retained; shared inputs, dialogs, menus and touch targets apply.

## Remaining local tabs

The remaining raw Tabs consumers serve message/template write-preview modes, the formula reference catalog, failure categories and the theme component preview. These are bounded choices within a task. The shared TabsList presents a native view selector on mobile and retains tabs on desktop. SegmentedControl follows the same mobile policy, including calendar, notification and editor controls. Goals and segments use native collection selectors. Timeline event types use a filter dialog with vertical checkboxes, explicit apply/cancel, a selected-type summary and reset; there is no horizontal event-type strip.

## Reference decisions

- [HubSpot record customization](https://knowledge.hubspot.com/object-settings/customize-records): distinguish properties, activity and associations.
- [Pipedrive detail view](https://support.pipedrive.com/en/article/detail-view): keep customer context close to operational actions and history.
- [Attio record redesign](https://attio.com/changelog/2026/record-page-redesign): use a clear relationship between essential data and activity.
- [Salesforce highlights](https://help.salesforce.com/s/articleView?id=platform.dynamic_highlights_panel.htm&language=en_US): prioritize explicit domain fields rather than arbitrary schema position.
- [Linear views](https://linear.app/docs/custom-views): treat collection views as work contexts.

## Verification status

Type checking, linting, production compilation and UI SDK declaration generation are the automated validation mechanisms. No test suite or scratch server was introduced.

Visual acceptance is pending. Browser discovery returned no available browsers. Do not treat compilation as a substitute for the requested desktop and mobile walkthrough.

When a browser is available, review 390, 768, 1280 and 1536 pixel layouts in both themes. Cover route navigation, long content, empty/error/loading states, saved filters after returning from records, record editing and dirty-state protection, mobile batch selection and sorting, stage movement, public pages, reduced permissions, disabled modules and legacy plugin contributions. Include a pass in English.


## Record workspace follow-up

Record details expose Overview, Tasks and Timeline as direct destinations on desktop, with complementary sections in a contextual menu. Overview no longer repeats the complete section directory on desktop; mobile retains that directory for sequential navigation. URLs and extension-provided sections remain supported.

Mobile record headers place labeled edit and action icons beside the record identity rather than reserving a separate full-width action row. Collection archive settings live in Display on desktop, including calendar mode; a visible reset control identifies non-default archive scopes. Mobile collections expose removable applied filter chips.

Static validation does not establish visual acceptance. The multi-resolution walkthrough remains pending.


## Completed structural follow-up

- Shared contextual workspaces retain a desktop section index and expose a disclosure with the active destination on mobile. They no longer stack the complete navigation before the content.
- Shared empty states use a compact footprint. Dialog footers place short actions beside each other while retaining wrapping and touch targets.
- Collection imports and exports use the additive `record.list.actions` slot. The existing toolbar slot remains available to extensions. Official built-in action contributions carry record-specific permission requirements.
- Deals keep creation in the page header and group saved views, pipeline and presentation under the current work context. Mobile has grouped stages, stage totals and an optional horizontal board. Custom-object boards show grouped columns on mobile and retain contextual movement without touch dragging.
- Dashboard keeps period selection visible and places scope/comparison in workspace options. Widget customization remains an explicit editing mode.
- Messaging moves status, assignment, channel and unread controls into workspace options. The list identifies its active status. The mobile conversation hides the redundant page heading.
- Automation editing opens on the flow. Name and description are disclosed on demand; activation and pause live in the action menu. Draft state remains in the parent.
- Quote actions choose Save, Send, Submit or Accept as the primary action according to the available state; other existing operations remain in a contextual menu.
- Proposal editing keeps editor and preview side by side on desktop and provides separate local modes on smaller screens without unmounting the draft.
- Distribution rules use dedicated new/edit pages with unsaved-change protection; history, simulation and deletion remain contextual. The dialog interface is retained for compatibility.
- Record creation can transfer entered values to `/records/:recordType/new`. Full creation preserves presets, validation, idempotency, creation events and module-declared collection cache invalidation. Long configured forms provide a desktop section index and mobile jump disclosure.

## Coverage boundaries

The route inventory covers core CRM records, custom objects, work queues, calendars, messaging, calls, booking, approvals, sales planning, reports, sequences, pricing, proposals, imports/exports, bulk history, duplicate resolution, privacy, themes, integrations, settings, authentication, installation and public pages. Shared changes apply to their actual consumers; specialized workflows are retained where their existing structure fits the task.

Source inspection and successful static checks are not interaction acceptance. No assertion is made that every route has been visually reviewed. Verify the route and permission matrix in a running demo once browser access exists, especially the new full-create transition, rule editor, legacy booking links, nested dialogs and keyboard behavior.
