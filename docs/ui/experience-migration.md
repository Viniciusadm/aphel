# Experience migration

## Decisions

The default audience is a general sales team. Navigation prioritizes daily work, customer context and next actions. Preserve Aphel branding, enabled-module boundaries, record permissions, saved views, custom fields, layout rules and existing REST contracts.

Desktop navigation groups work, CRM, operations, insights and administration. Sidebar search includes contributed group labels. Personal ordering remains effective within the new categories. Mobile exposes Today, Deals when permitted, Search and Menu; quick entry lives in the header create menu, alongside the standard creation actions. No floating creation button overlays list content or pagination. Page actions occupy a separate full-width row below the title on mobile. Pagination contains only Previous, Page X of Y and Next. Result ranges are rendered beside the list, and page-size settings belong in Display. Space is reserved only for the bottom navigation.

## Coverage inventory

- Shell, dashboard, search and following: shared page hierarchy, searchable grouped navigation and mobile destinations.
- Contacts, companies, leads, deals, activities, products and custom objects: shared record overview, domain field priorities, full detail destination, contextual collections and dedicated editing.
- Record collections, segments and relation lists using RecordListView or RecordTable: mobile cards, sorting, selection, row actions and saved-view behavior. Specialized comparison tables retain contained horizontal scrolling.
- Deals: desktop board and mobile stage selection, including explicit stage and pipeline movement, validation and existing approval flows.
- Tasks: related route views through AreaNavigation. Calendar keeps its purpose-specific representations and existing mobile agenda behavior.
- Messaging: existing desktop list/conversation and mobile sequential conversation flow retained. Shared shell, controls and feedback apply.
- Scheduling: bookings are the default workspace; availability and meeting-type configuration are contextual sections with URL selection.
- Calls: sessions remain operational; reporting is independently discoverable under Insights; outcomes belong to organization settings. The previous outcomes URL redirects.
- Reports: library entry, permission-filtered built-in destinations, saved reports and dedicated report builder.
- Automations: flow, simulation, execution history and versions are contextual destinations; draft state and unsaved changes protection remain in the parent editor.
- Email: mailbox management, templates, organization blocklist and preferences use contextual sections. Write/preview modes remain local tabs.
- Approvals: native queue and status selectors replace navigation-shaped filters.
- Price lists, privacy, configuration packages and theme editing: independent functions use contextual workspaces. Theme preview retains a small mode selector.
- Sequences, proposals, quotes, lead forms, imports, exports, bulk history, duplicates and merge flows: existing collection/editor or sequential workflows retained; shared navigation, record contributions and controls are migrated.
- Goals, forecast, scoring, distribution, currencies, pipelines, tags and specialized modules: shared shell and controls apply; contributed record content uses the section API.
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

Visual acceptance is pending. Browser discovery returned no available browsers, and creating an in-app browser tab returned `Browser is not available: iab`. Do not treat compilation as a substitute for the requested desktop and mobile walkthrough.

When a browser is available, review 390, 768, 1280 and 1536 pixel layouts in both themes. Cover route navigation, long content, empty/error/loading states, saved filters after returning from records, record editing and dirty-state protection, mobile batch selection and sorting, stage movement, public pages, reduced permissions, disabled modules and legacy plugin contributions. Include a pass in English.
