---
title: Screens and contributions
description: The complete registration API of a web module, with every contribution point, slot, filter and building block.
sidebar:
  order: 51
---

A web bundle registers everything it adds to the application inside `register(app)`. This page documents each method of `app`, the slots and filters you can use, the shared building blocks for record screens, and the rules that keep your hooks stable. Packaging, shared modules and translations are covered in [Web bundle](/_aphel/docs/plugins/web-bundle/).

## Conventions used by every contribution

**Permissions.** Wherever a contribution accepts `permission`, the value is a string or an array of strings. An array means any of them: the contribution is shown when the user has at least one. An empty array or no value means no check. The check uses the permissions of the signed-in user. Hiding a screen is a convenience only. The API enforces permissions on its own.

**Labels.** Every `label`, `description` and `group.label` is a translation key resolved in your module namespace (the plugin key). A missing key is shown as written. A key with a prefix, such as `layout:nav.groups.work`, is looked up in that namespace.

**Record type selectors.** `recordTypes` and similar properties accept a `RecordTypeSelector`:

| Form | Matches |
| --- | --- |
| `'deals.deal'` | That record type. |
| `'deals.*'` | Every record type that starts with `deals.`. |
| `'*'` | Every record type. |
| `['deals.deal', 'people.*']` | Any of the entries. |
| `(recordType) => boolean` | Whatever the function accepts. |

Plugin entities have record types of the form `<plugin key>.<entity>`. A contribution that omits `recordTypes` matches everything, including places that have no record type.

**Module activity.** Contributions of a plugin are active while the plugin is enabled. Ordering is by `order` ascending (default `1000` unless noted), then by registration order.

## The `app` object

| Method | Purpose |
| --- | --- |
| `app.key` | The module key (read-only). |
| `routes(routes, options?)` | Add routes inside the signed-in area. |
| `publicRoutes(routes)` | Add routes that need no sign-in. |
| `nav(item)` | Add a navigation entry. |
| `navSection(section)` | Declare a navigation section. |
| `settings(area, item)` | Add a page to a settings area. |
| `settingsArea(area)` | Declare a settings area. |
| `settingsGroup(area, group)` | Add a group to a settings area. |
| `recordTab(recordTypes, tab)` / `recordSection(recordTypes, section)` | Add a tab or section to record detail pages. |
| `recordList(extension)` | Extend record lists. |
| `traitListView(view)` | Add a list view for object types with a trait. |
| `slot(name, contribution)` | Render a component in a named place. |
| `filter(name, fn, priority?)` | Transform a value computed by the application. |
| `provideComponent(key, component)` | Provide a contract component. |
| `override(id, component)` | Replace a component that the application marks overridable. |
| `fieldType(type, renderers)` | Custom input and display for a field type. |
| `timelineRenderer(kind, renderer)` / `eventRenderer(eventType, renderer)` | Render timeline entries. |
| `timelineSource(factory)` | Replace the data source of record timelines. |
| `widget(key, renderer)` | Render a dashboard widget declared in the manifest. |
| `command(command)` | Add a command palette entry. |
| `keybinding(keybinding)` | Add a keyboard shortcut action. |
| `keybindingScope(scope)` | Declare where a group of shortcuts applies. |
| `createAction(action)` | Add an entry to the create menu. |
| `quickAdd(action)` | Add a mobile quick-add action. |
| `referenceOptions(kind, provider)` | Provide options for a reference kind. |
| `configPackageLabels(labels)` | Labels for configuration package screens. |
| `ruleError(code, formatter)` | Format a rule violation error code. |
| `setupStep(step)` / `setupTask(task)` | Contribute to the installation wizard. |
| `on(name, handler)` | Listen to a client-side event. |

### Routes

```ts
routes(routes: RouteObject[], options?: { layout?: 'app' | 'bare' }): void
publicRoutes(routes: RouteObject[]): void
```

Routes are React Router 7 `RouteObject`s. Write paths without a leading slash (`'deal-watch'`); they are attached at the root of the application. Use `element`, `Component`, `lazy`, `children`, `loader` and the other route properties.

| Registration | Requires sign-in | Chrome |
| --- | --- | --- |
| `routes(r)` or `routes(r, { layout: 'app' })` | Yes | Rendered inside the application frame (navigation, top bar). |
| `routes(r, { layout: 'bare' })` | Yes | No frame, for full-screen pages. The organization theme and the `app.shell` slot still render, so background contributions keep running. |
| `publicRoutes(r)` | No | No frame and no session. |

Plugin routes are listed before the built-in settings routes, so avoid paths that already exist. Route errors are caught by an error boundary around the application area.

A plugin whose pages are plain static files and need no sign-in can use [Public web apps](/_aphel/docs/plugins/public-web-apps/) instead.

### Navigation

```ts
nav(item: NavContribution): void
navSection(section: { key: string; label: string; order?: number }): void
```

| `NavContribution` property | Type | Meaning |
| --- | --- | --- |
| `to` | `string` | Required. Target path, for example `'/deal-watch'`. |
| `label` | `string` | Required. Translation key. |
| `icon` | `LucideIcon` | Required. |
| `key` | `string` | Stable identifier used for the user's order and hidden-item preferences. Default: the path without the leading slash (`'home'` for `/`). |
| `end` | `boolean` | Match the path exactly when marking the entry active. |
| `permission` | `PermissionRequirement` | Hide the entry without it. |
| `badge` | `ComponentType<{ collapsed?: boolean }>` | Rendered next to the label, for counters. |
| `order` | `number` | Default `600` (`NAV_ORDER.modules`). `NAV_ORDER` also has `customObjects: 500` and `administration: 900`. Entries below 500 appear before the custom object entries. |
| `group` | `{ key, label, icon }` | Collapsible group that holds several entries with the same group key. |
| `section` | `string` | Section of the sidebar. Built-in sections: `work`, `records`, `operations` (default), `analysis`, `administration`. A blueprint can move a plugin into its own section. |
| `mobile` | `boolean` | Pin this entry in the mobile bottom bar. Only the first entry with `mobile: true` is used. |

If two entries share a label, the group name is prefixed to tell them apart. The plugin manifest option `[menu] flat = true` removes your entries from groups. Users can reorder or hide entries in their preferences; the "Settings" entry is always last and cannot be hidden.

`navSection` declares a section. Sections with the same key are merged and the first definition wins.

### Settings pages

```ts
settings(area: SettingsAreaKey, item: SettingsContribution): void
settingsArea(area: SettingsAreaContribution): void
settingsGroup(area: SettingsAreaKey, group: { key: string; label: string; order?: number }): void
```

The built-in areas and their groups:

| Area key | Base path | Groups |
| --- | --- | --- |
| `account` | `/account` | `profile`, `communication`, `accountSecurity` |
| `organization` | `/settings` | `organization`, `records`, `communication` |
| `integrations` | `/integrations` | `connections`, `webhooks` |
| `data` | `/data` | `transfer`, `compliance` |

`SettingsContribution`:

| Property | Type | Meaning |
| --- | --- | --- |
| `group` | `string` | Required. A group key of the area, or a new key. An unknown key creates the group with the label `<your key>:<group>`, so define that key in your translations or declare it with `settingsGroup`. |
| `path` | `string` | Required. The page is served at `<area base path>/<path>`. |
| `label`, `description` | `string` | Required. Translation keys. |
| `icon` | `LucideIcon` | Required. |
| `permission` | `string` | A single permission. |
| `order` | `number` | Sort order inside the group. |
| `element` / `lazy` | `ReactNode` / `RouteObject['lazy']` | The page. One of them, unless you only use `routes`. |
| `routes` | `RouteObject[]` | Extra child routes, for sub-pages. |

`SettingsAreaContribution` adds a new area: `key`, `basePath`, `titleKey`, `descriptionKey` (both resolved in the `settings` namespace, so write them as `your_key:area.title`), `icon`, `scope` (`'account'` or `'admin'`, default `'admin'`), `primary`, `order` and `groups`. An area appears only for users who can see at least one of its pages.

### Record detail: tabs and sections

```ts
recordTab(recordTypes: RecordTypeSelector, tab: RecordTabContribution): void
recordSection(recordTypes: RecordTypeSelector, section: RecordSectionContribution): void
```

Both register the same kind of entry. `RecordSectionContribution` adds an optional `summary` component that is shown on the overview of the record.

| Property | Type | Meaning |
| --- | --- | --- |
| `key` | `string` | Required. Unique among the tabs of the record; also the value of the `tab` URL parameter. |
| `label` | `string` | Required. Translation key. |
| `icon` | `LucideIcon` | Optional. |
| `permission` | `PermissionRequirement` | Hide the tab without it. |
| `order` | `number` | Sort order among module tabs. |
| `group` | `string` | Optional grouping label. `"primary"` shows the tab in the tab bar next to the overview and timeline; other tabs are listed under "Sections". |
| `useVisible` | `(props: RecordTabVisibilityProps) => boolean` | A hook that decides visibility. See below. |
| `render` | `ComponentType<RecordTabRenderProps>` | Required. The tab content. |
| `summary` | `ComponentType<RecordTabRenderProps>` | Sections only. |

`RecordTabRenderProps` is `{ recordType: string; recordId: string; readOnly: boolean }`. `RecordTabVisibilityProps` adds `enabled: boolean`.

Each tab is wrapped in an error boundary, so a crash in your tab affects only that tab.

**Hook rules for `useVisible`.** The application calls the `useVisible` function of every registered tab on every render of a record page, in a fixed order, including tabs of record types that do not match and tabs whose module is not active. The function is a React hook:

- It must call the same hooks in the same order every time, with no early return before a hook.
- It receives `enabled`. When `enabled` is false the tab does not apply, so skip work: pass `enabled` to `useQuery({ enabled })` and similar.
- It must return a boolean quickly and must not throw.

The same rules apply to `recordList(...).use` and `referenceOptions(...).use`.

### Record lists

```ts
recordList(extension: RecordListExtension): void
```

```ts
interface RecordListExtension {
  key: string
  recordTypes?: RecordTypeSelector
  permission?: PermissionRequirement
  use: (props: { recordType: string; schema: FieldSchema; records: RecordItem[]; enabled: boolean }) => {
    filterFields?: FieldDefinition[]
    titleAddon?: (record: RecordItem) => ReactNode
  }
}
```

`use` is a hook called for every registered extension on every list render (see the hook rules above). `filterFields` adds fields to the filter builder, skipping keys that already exist. `titleAddon` renders extra content next to the title cell of each row.

```ts
traitListView(view: { trait: string; key: string; label: string; icon: LucideIcon; render: ComponentType<{ recordType: string; traitConfig: unknown; search: string; canEdit: boolean }> }): void
```

`traitListView` adds an alternative view to the list of custom object types whose entity declares the given trait.

### Slots

```ts
slot<N extends SlotName>(name: N, contribution: SlotContribution<SlotProps<N>>): void
```

`SlotContribution`:

| Property | Meaning |
| --- | --- |
| `render` | Required. A component that receives the slot props. |
| `key` | Optional stable key. Without it, the registration number is used. |
| `recordTypes` | Only for slots whose props contain `recordType`. A slot with a selector never renders where there is no record type. |
| `permission` | Hide without it. |
| `order` | Sort order. |

Slots defined by the application (`SlotPropsMap`):

| Slot | Props | Rendered in |
| --- | --- | --- |
| `record.detail.title` | `RecordSlotProps` | Next to the record title. |
| `record.detail.header` | `RecordSlotProps` | Below the record header. |
| `record.detail.actions` | `RecordSlotProps` | In the actions menu of the record. |
| `record.detail.sidebar` | `RecordSlotProps` | Below the fields panel. |
| `record.fields.panel` | `RecordSlotProps` | Inside the fields panel. |
| `record.list.toolbar` | `{ recordType, filter: FilterNode \| null, applyFilter(filter) }` | List toolbar. |
| `record.list.actions` | `{ recordType }` | List header actions. |
| `record.list.viewActions` | `{ recordType, viewId }` | Actions of the active saved view. |
| `record.list.rowActions` | `{ recordType, record: RecordItem }` | An actions cell on every row. |
| `record.list.bulkActions` | `BulkActionContext` | Bulk actions bar; adding one enables row selection. |
| `record.form.sections` | `{ recordType?, mode: 'create' \| 'edit', record? }` | End of the record form. |
| `composer.tools` | `{ context: 'note' \| 'email' \| 'message', subject?, value, onChange(value) }` | Tools in text composers. |
| `user.menu` | none | The user menu. |
| `app.topbar` | none | The top bar. |
| `modules.settings` | none | The modules page of the settings. |
| `app.shell` | none | Once, at the root of the signed-in frame. |
| `app.sidebar` | `{ collapsed?, onNavigate? }` | Below the sidebar navigation. |

`RecordSlotProps` is `{ recordType: string; record: RecordItem; readOnly: boolean }`. `BulkActionContext` is `{ recordType, selectedIds: string[], allMatching: boolean, total: number, query: { filter, search, archived }, clearSelection() }`.

Built-in modules define further slots with a `<module>.` prefix (for example `deals.badges` or `reports.library`). Their props are not part of the public SDK types, so rely only on the slots in the table above.

You can also host slots in your own screens with `<Slot name="record.list.toolbar" props={...} />`, which renders the components that other modules contributed.

### Filters

```ts
filter<N extends FilterName>(name: N, fn: (value, context) => value, priority?: number): void
```

A filter receives the current value and a context, and returns a new value. Filters for the same name run in ascending `priority` (default `10`), then in registration order, each receiving the result of the previous one. Return a new value instead of mutating the input.

| Name | Value | Context |
| --- | --- | --- |
| `records.columns` | `FieldDefinition[]` | `{ recordType }` |
| `records.formLayout` | `FormLayout \| null` | `{ recordType?, mode }` |
| `records.detailTabs` | `RecordTab[]` | `{ recordType, record, readOnly }` |
| `records.filterFields` | `FilterFieldExtension[]` | `{ recordType }` |
| `navigation.items` | `NavigationEntry[]` | none |
| `navigation.customObjects` | custom object list | none |
| `commands` | `PaletteCommand[]` | none |
| `settings.sections` | `SettingsMenuGroup[]` | `{ area }` |
| `dashboard.eventLabel` | `ReactNode` | `{ event: { event_type, payload } }` |

`records.detailTabs` runs after the tabs of all modules are collected, so it can reorder or remove any tab.

### Contract components and overrides

```ts
provideComponent<K extends ContractComponentKey>(key: K, component: ComponentType<ContractComponentProps<K>>): void
override<P extends object>(id: string, component: ComponentType<P>): void
```

A contract component is a component with a fixed props shape, identified by a key. The host renders `<ContractComponent name="..." />` and uses the first active provider, in registration order. Built-in modules register before plugins, so a plugin provider is used only when no built-in provider exists for that key.

| Key | Props | Used by |
| --- | --- | --- |
| `app.Frame` | `AppFrameProps` | The application frame. Selected by the chosen interface plugin only. |
| `records.Presentation` | `RecordPresentationProps` | The record detail page. Selected by the chosen interface plugin only. |
| `notifications.MentionInput` | textarea props with `value: string` and `onValueChange?(value)` | Note editor. |
| `workflow.RottingBadge` | `{ record: object; className? }` | Deal lists. |
| `dashboard.PeriodSelector` | `{ value: LayoutPeriod; onChange(period) }` | Reports. |
| `dashboard.CompareToggle` | `{ checked: boolean; onChange(checked) }` | Reports. |

The first two are the replaceable screens of [UI plugins](/_aphel/docs/plugins/ui-plugins/). Use `useContractComponent(key, Fallback?)` or `<ContractComponent name={key} {...props} />` in your own code to render whatever is provided.

An override replaces a component that the application renders through `<Overridable id=...>`. The last registered override wins, and in development builds a console warning names the winner.

| Id | Props | Replaces |
| --- | --- | --- |
| `people.ListPage` | none | The contacts list page. |
| `people.DetailPage` | none | The contact detail page. |
| `organizations.ListPage` | none | The companies list page. |
| `organizations.DetailPage` | none | The company detail page. |
| `leads.Page` | none | The leads page. |
| `leads.ConvertDialog` | `{ lead: { id: string; name: string } \| null; onOpenChange(open: boolean) }` | The lead conversion dialog. |

These ids exist only when the owning built-in module is part of the installation.

### Field types

```ts
fieldType(type: string, renderers: { Input?: ComponentType<FieldInputProps>; Display?: ComponentType<FieldDisplayProps> }): void
```

`type` is a field type name such as `text`, `long_text`, `number`, `currency`, `date`, `datetime`, `select`, `multi_select`, `checkbox`, `url`, `email`, `phone`, `user`, `record`, `references`, `file`, `image`, `document` or `formula`. Renderers you register override the built-in ones for that type, the last registration winning. Writing `'<type>:<field key>'` targets only the field with that key, and it wins over the type-wide renderer.

`FieldInputProps`: `{ field, value, onChange(value), initialValue?, id?, onBlur?, disabled?, invalid?, autoFocus?, className?, 'aria-describedby'?, 'aria-labelledby'?, 'aria-label'? }`. `FieldDisplayProps`: `{ field: FieldDefinition; value: unknown }`.

### Timeline, widgets, commands and the rest

| Method | Details |
| --- | --- |
| `timelineRenderer(kind, { icon, render(item) })` | Renders timeline items of a `kind`. `item` is `{ id, kind, occurred_at, actor, title, body?, record?, metadata }`. The last registration wins. |
| `eventRenderer(eventType, renderer)` | Same, keyed by the event type. It takes priority over the renderer of the kind. |
| `timelineSource(factory)` | `factory(recordType, recordId)` returns `{ fetchPage({ cursor, kinds }), kindOptions?, fetchKindOptions? }`. The last registration wins. |
| `widget(key, { render(data), isEmpty?(data), settings? })` | Renders a dashboard widget. The key is `<plugin key>_<widget name>` from the manifest. `settings` lists `{ kind: 'integer', key, label, min, max }` or `{ kind: 'boolean', key, label }` entries. |
| `command({ id, label, icon?, keywords?, permission?, to?, run?(helpers), keys? })` | A command palette entry. Use `to` to navigate or `run({ navigate })` to act. `keys` also registers a keyboard shortcut with the command `id` (see [Keyboard shortcuts](#keyboard-shortcuts)) and shows it next to the entry. |
| `createAction({ id, label, icon, permission?, recordType?, targetRecordTypes?, linkedLabel?, order?, render({ open, target?, onClose }) })` | An entry in the create menu. `render` draws your dialog. `recordType` makes it the contextual action on that type's list page. `targetRecordTypes` also offers it on matching record pages, labelled `linkedLabel`, with `target` set to that record. |
| `quickAdd({ key, label, title?, icon, permission?, order?, linksRecord?, recordShortcut?, render({ initialRecord, onDone }) })` | An action in the mobile quick-add sheet. `linksRecord` defaults to true. |
| `referenceOptions(kind, { use(context), queryKey? })` | Provides `{ options: { value, label, group? }[], isLoading? }` for a kind such as `tag`, `pipeline_stage` or `sequence`. `use` is a hook called for every provider. The last active provider wins. |
| `configPackageLabels({ sections?, kinds?, properties? })` | Maps keys to labels. |
| `ruleError(code, formatter)` | `formatter(messages)` returns the text for an API error with that code and field messages. |
| `setupStep(...)`, `setupTask(...)` | Contribute to the installation wizard. Plugins are installed after the wizard has finished. |
| `on('record.created', handler)` | `handler({ recordType, record })`. Fired when a record is created through the record form dialog and the contact quick form, not for every creation. |

### Keyboard shortcuts

Every shortcut in Aphel, from the core and from plugins, goes through one registry and one key listener. People change them in **My account > Keyboard shortcuts**, administrators set company defaults in **Settings > Keyboard shortcuts**, and both screens warn about conflicts. A plugin declares *actions* with default keys; it never listens to `keydown` on `window` itself.

```tsx
register(app) {
  app.keybindingScope({ id: 'deal_watch.board', label: 'keys.scope', kind: 'overlay' })
  app.keybinding({ id: 'deal_watch.go', label: 'nav.board', to: '/deal-watch', keys: ['g w'] })
  app.keybinding({ id: 'deal_watch.refresh', label: 'keys.refresh', scope: 'deal_watch.board', keys: ['f5', 'r'] })
}
```

| `KeybindingContribution` property | Type | Meaning |
| --- | --- | --- |
| `id` | `string` | Required. Stable id, prefixed with your plugin key. Users' and the company's overrides are stored under it. |
| `label` | `string` | Required. Translation key. With `to`, the settings screen shows it as "Go to {label}". |
| `keys` | `string[]` | Required. Default key combinations. `[]` ships the action without a default. |
| `scope` | `string` | Where the action applies. Default `global`. |
| `description` | `string` | Translation key shown under the label in settings. |
| `permission` | `PermissionRequirement` | Hide the action without it. |
| `to` | `string` | Navigate to this path. No handler needed. |
| `run` | `({ navigate }) => void` | Static handler. Use it or `to` when the action does not depend on screen state. |
| `allowInEditable` | `boolean` | Also fire while the focus is in a text field. Keys with `Ctrl`/`Cmd`/`Alt` and `F1`–`F24` always do. |
| `repeat` | `boolean` | Fire again while the key is held down. |
| `always` | `boolean` | Fire even with shortcuts turned off and inside dialogs. Reserved for the command palette. |

Key combinations are lower case, modifiers first, joined by `+`: `mod+k`, `alt+d`, `shift+f2`, `f9`, `delete`, `arrowup`, `plus`, `?`. `mod` is `Cmd` on a Mac and `Ctrl` elsewhere. A space separates the two steps of a sequence such as `g w` (press `g`, then `w` within about a second). Named keys: `enter`, `escape`, `space`, `tab`, `backspace`, `delete`, `insert`, `home`, `end`, `pageup`, `pagedown`, `arrowup`, `arrowdown`, `arrowleft`, `arrowright`, `plus` and `f1`–`f24`.

**Scopes.** `global` actions work on every screen. A scope of kind `overlay` is active while a screen calls `useKeybindingScope(id)`; its actions win over global ones with the same keys. A scope of kind `isolated` belongs to one element, such as a text box or a dialog, and its actions only fire when that element handles the event through `useKeybindingProps`. The kernel declares the isolated scope `editor` for text boxes that send or save with `mod+enter`.

**Handlers.** Import the hooks from `@aphel/web-sdk`:

| Hook or component | Use |
| --- | --- |
| `useKeybindingScope(scope, enabled?)` | Activate an overlay scope while the component is mounted. |
| `useKeybinding(id, handler, { enabled? })` | Run `handler(event)` for a global or overlay action. |
| `useKeybindingHandlers({ [id]: handler }, { enabled? })` | The same for several actions. |
| `useKeybindingProps({ [id]: handler })` | Returns `{ onKeyDown }` to spread on the element of an isolated scope. |
| `useActionKeys(id)` | The keys that apply now, after the company and user overrides. |
| `<KeybindingHint action="id" />` / `<KeybindingKeys keys={[...]} />` | Show the current keys with the shared `Kbd` style. `chordText(chord)` gives plain text such as `Ctrl+Enter` or `⌘↵`. |

A handler that returns `false` lets the key go through, for example when the cart is empty or the user is typing. Any other return value consumes the key.

```tsx
function Board() {
  useKeybindingScope('deal_watch.board')
  useKeybinding('deal_watch.refresh', () => void refetch())
  return <Button onClick={() => void refetch()}>{t('refresh')} <KeybindingHint action="deal_watch.refresh" /></Button>
}
```

**Conflicts.** The same keys in the same scope, or keys where one starts the other (`g` and `g w`), are errors: the settings screens flag them and, in development, the console warns when defaults collide. An overlay or isolated action that reuses a global key is only a warning, because the more specific scope wins where both apply. Keys usually taken by the browser, such as `mod+w` or `f5`, are also warnings.

## Building blocks for record screens

These are exported from `@aphel/web-sdk`.

### `RecordListView`

```tsx
<RecordListView recordType="deals.deal" baseFilter={filter} />
```

| Prop | Type | Meaning |
| --- | --- | --- |
| `recordType` | `string` | Required. |
| `emptyAction` | `ReactNode` | Shown in the empty state. |
| `toolbarActions` | `ReactNode` | Extra toolbar content. |
| `bulkActions` | `(context: BulkActionContext) => ReactNode` | Enables selection and renders bulk actions. |
| `onRowClick` | `(record) => void` | Overrides navigation to the detail page. |
| `rowClassName` | `(record) => string \| undefined` | |
| `titleAddon` | `(record) => ReactNode` | Content next to the title cell. |
| `baseFilter` | `FilterNode` | A filter always applied, combined with the user's filters by AND. |
| `archived` | `'exclude' \| 'include' \| 'only'` | Archived records mode. |
| `archivedFilter` | `boolean` | Let the user choose the mode through the URL. |
| `className` | `string` | |

It provides search, saved views, column choice, sorting, filters, pagination and a calendar layout when the type has date fields. The state lives in the URL. The record type must be registered, otherwise the component throws `record resource not registered`. Built-in types are registered at startup. Types that come from plugin entities and custom objects are registered when `useCustomObjects()` has loaded, so wait for it before rendering the list, as the example in [Examples](/_aphel/docs/plugins/examples/) does.

### `RecordDetailPage`

```tsx
<RecordDetailPage recordType="deals.deal" recordId={id} back={{ to: '/deals', label: 'Deals' }} />
```

| Prop | Type | Meaning |
| --- | --- | --- |
| `recordType`, `recordId` | `string` | Required. |
| `actions` | `ReactNode \| (record) => ReactNode` | Extra header actions. |
| `back` | `{ to: string; label?: string }` | Back link. |
| `fieldKeys` | `string[]` | Fields to show. |
| `readOnly` | `boolean` | |
| `readOnlyFieldKeys` | `string[]` | Defaults to `id`, `created_at`, `updated_at`, `archived_at`, `deleted_at`. |
| `ownerField` | `string` | Default `owner_id`. |
| `tabs` | `RecordTab[] \| (defaults) => RecordTab[]` | Replace or adjust the default tabs (`timeline`, `tasks`, `notes`, `files`, `related`). |
| `timeline` | `TimelineSource` | |
| `defaultTab` | `string` | |
| `className` | `string` | |

It loads the record, renders loading, error and not-found states, and renders the page through the active `records.Presentation`. Tabs registered with `recordTab` are appended to the default tabs.

### Other pieces

| Component or hook | Use |
| --- | --- |
| `RecordForm` | Form for a record. Props: `schema`, `fields`, `layout` (field keys), `formLayout`, `readOnlyKeys`, `record`, `defaultValues`, `mode` (`'create' \| 'edit'`), `onSubmit(payload, values)`, `onCancel`, `onDirtyChange`, `onValuesChange`, `submitLabel`, `columns` (`1 \| 2`), `guardUnsavedChanges`, `disabled`, `id`, `className`. |
| `RecordFormDialog` | The same form in a dialog. Props: `recordType`, `open`, `onOpenChange`, `layout`, `defaultValues`, `presetValues`, `record`, `title`, `description`, `onCreated(record)`, `onUpdated(record)`. |
| `KanbanBoard` | Board of a select field. Props: `recordType`, `field: FieldDefinition`, `search`, `canEdit`, `columnOptions?: { value, label, color? }[]`. |
| `Timeline` | Record timeline renderer for a `TimelineSource`. |
| `useRecord(recordType, id)`, `useUpdateRecord(recordType, id)` | Query and mutation for one record. |
| `useFieldSchema(recordType)` | Field schema query. |
| `useRecordPageModel(options)`, `RecordHeader`, `RecordFieldsPanel`, `RecordTabs`, `RecordPageShell` | The parts of the record page, used to build a [custom presentation](/_aphel/docs/plugins/ui-plugins/). |
| `useChangeFeed(pluginKey, onChange, options?)` | Live change stream of a plugin. See [Testing and debugging](/_aphel/docs/plugins/testing-and-debugging/). |
| `api` | `get`, `post`, `put`, `patch`, `delete`. Paths are relative to `/api`, for example `api.get('/v1/people')`. Sends the CSRF header and cookies, unwraps `{ data }` envelopes and throws `ApiError`. |

## Permissions and gating

- Use `permission` on navigation, tabs, slots, commands and actions. In your own components, call `useSession()` and `hasPermission('deals.update')`, or wrap content in `RequirePermission`.
- The permissions you reference must exist in the role catalog: built-in permissions, or those you declare in `[[permissions]]` or through your entities. See [Permissions and roles](/_aphel/docs/plugins/permissions-and-roles/).
- Pages of a plugin are available only while it is enabled. Disabling or removing a plugin removes its routes and contributions on the next page load.

## Related pages

- [Web bundle](/_aphel/docs/plugins/web-bundle/)
- [UI plugins](/_aphel/docs/plugins/ui-plugins/)
- [UI library](/_aphel/docs/plugins/ui-library/)
- [Examples](/_aphel/docs/plugins/examples/)


## Local printing

The web SDK exports `sendLocalPrintJob(configuration, job)` for execution in the cashier browser. The WASM backend must persist/authorize the domain print job and return it to the appropriate station; a remote worker cannot reach that computer via its own localhost. The helper accepts only an exact loopback origin, uses a station-local Bearer token, sends a stable `Idempotency-Key` and validates the matching job acknowledgement. It does not retry or open a browser print dialog automatically after an unknown outcome.

```ts
import { sendLocalPrintJob } from '@aphel/web-sdk'
const ack = await sendLocalPrintJob(
  { url: 'http://127.0.0.1:9876', token: stationToken },
  { id: persistedJob.id, target: 'cashier', payload: receiptHtml },
)
// accepted/duplicate acknowledges delivery, not physical printing.
```

The local agent must implement origin-restricted CORS (including preflight), authentication, durable idempotency and job-status lookup. Browser local-network permissions must be tested on the target devices. This transport helper is not an ESC/POS driver or a backend print queue. See the installation plan for the proposed agent protocol.
