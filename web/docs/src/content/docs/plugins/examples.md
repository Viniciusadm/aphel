---
title: Examples
description: Three complete plugins with every file, from a colors-only interface to a plugin that replaces the application frame and the deal page.
sidebar:
  order: 57
---

Each example lists every file of the project. The manifests were checked with the installer validators, the packages were built with `aphel-plugin pack`, and the TypeScript was type checked against the SDK. Adjust the relative paths to your Aphel checkout where noted.

## Example 1: a colors-only interface

A plugin that only changes the look of Aphel. There is no code, so the project is one file.

```text
midnight/
  plugin.toml
```

`plugin.toml`:

```toml
key = "midnight"
name = "Midnight"
version = "1.0.0"
description = "A dark-slate interface with a teal accent."
api = 1
requires_aphel = ">=0.1.0"

[ui]
label = "Midnight"
description = "Slate surfaces and a teal accent, with sharper corners."

[ui.tokens.shared]
radius = "0.375rem"
card-shadow = "none"
font-family-heading = { families = ["Georgia"], fallback = "serif" }

[ui.tokens.light]
background = "#f4f6f8"
foreground = "#0f1b24"
card = "#ffffff"
primary = "#0f766e"
primary-solid = "#0f766e"
primary-foreground = "#ffffff"
border = "#d5dde3"
sidebar = "#0f1b24"
sidebar-foreground = "#d5dde3"
sidebar-accent = "#1d2f3c"
sidebar-accent-foreground = "#ffffff"
sidebar-border = "#1d2f3c"

[ui.tokens.dark]
background = "#0b141b"
foreground = "#e6edf1"
card = "#111d26"
primary = "#2dd4bf"
border = "#1d2f3c"
```

What it does:

- `key`, `name`, `version`, `api` and `requires_aphel` are the compatibility fields. `api = 1` is required and `requires_aphel` limits the Aphel versions the package installs on.
- `[ui]` makes it an interface plugin. `label` (1 to 60 characters) is what users see on the Appearance page.
- `[ui.tokens.shared]` applies in every mode. `radius` is a length, `card-shadow` is a shadow (`none` is allowed) and `font-family-heading` is a font token with a system font and a generic fallback.
- `[ui.tokens.light]` sets the colors. Hex values are valid colors. `primary` is the text and outline accent, `primary-solid` the button fill and `primary-foreground` the text on it. The organization brand color still overrides these three when one is set.
- `[ui.tokens.dark]` is validated and served, but the application is currently light only, so it has no visible effect yet.
- The key `midnight` is allowed because it is not `core` or `default`.

Build and install:

```bash
aphel-plugin pack midnight -o midnight-1.0.0.aphelplugin
```

Install the file from Integrations, then Plugins. As a UI plugin without code it asks for no code acknowledgement. Then choose Midnight in Account, Appearance.

## Example 2: a web-only plugin with a nav entry, a list page and a record tab

A plugin with no backend that adds a "Deal watch" page listing open deals and a "Facts" tab on the deal page.

```text
deal-watch/
  plugin.toml
  web/
    package.json
    tsconfig.json
    vite.config.ts
    src/
      index.tsx
      WatchPage.tsx
      FactsTab.tsx
      translations.ts
      style.css
```

`plugin.toml`:

```toml
key = "deal_watch"
name = "Deal Watch"
version = "0.1.0"
description = "A watch list of open deals and a facts tab on the deal page."
api = 1
requires_aphel = ">=0.1.0"

[web]
entry = "web/index.js"
style = "web/style.css"
```

`[web]` declares the bundle and its stylesheet. Nothing else is needed: there is no backend, no `requires` and no entities. Because the plugin has web code, installing needs `acknowledge_web_version=0.1.0`.

`web/package.json`:

```json
{
  "name": "aphel-deal-watch-web",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "vite build",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {
    "lucide-react": "^1.59.0"
  },
  "devDependencies": {
    "@types/react": "^19.3.0",
    "@types/react-dom": "^19.3.0",
    "typescript": "~7.0.2",
    "vite": "^8.3.4"
  }
}
```

The `build` script is what `aphel-plugin pack` runs. Only `lucide-react` is bundled. React and the SDK are external.

`web/vite.config.ts`:

```ts
import { defineConfig } from 'vite'

export default defineConfig({
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    lib: {
      entry: 'src/index.tsx',
      formats: ['es'],
      fileName: () => 'index.js',
      cssFileName: 'style',
    },
    rollupOptions: {
      external: [
        'react',
        'react/jsx-runtime',
        'react-dom',
        'react-dom/client',
        'react-router',
        '@tanstack/react-query',
        'react-i18next',
        '@aphel/web-sdk',
        '@aphel/ui',
      ],
    },
  },
})
```

A library build with one ES module output named `index.js` and one stylesheet named `style.css`, which become `web/index.js` and `web/style.css` in the package. The `external` list is the set of shared modules.

`web/tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "es2023",
    "lib": ["ES2023", "DOM", "DOM.Iterable"],
    "module": "esnext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true,
    "verbatimModuleSyntax": true,
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "types": ["vite/client"],
    "paths": {
      "@aphel/web-sdk": ["../../aphel/web/src/sdk/index.ts"],
      "@aphel/ui": ["../../aphel/sdk/js/aphel-ui/dist/sdk/ui.d.ts"],
      "@/*": ["../../aphel/web/src/*"]
    }
  },
  "include": ["src"]
}
```

The `paths` point at the SDK sources in an Aphel checkout two levels above the project.

`web/src/index.tsx`:

```tsx
import { defineModule } from '@aphel/web-sdk'
import { Binoculars, Eye } from 'lucide-react'
import './style.css'
import { FactsTab } from './FactsTab'
import { translations } from './translations'
import { WatchPage } from './WatchPage'

export default defineModule({
  key: 'deal_watch',
  translations,
  register(app) {
    app.routes([{ path: 'deal-watch', element: <WatchPage /> }])
    app.nav({
      to: '/deal-watch',
      label: 'nav.title',
      icon: Binoculars,
      permission: 'deals.read',
      section: 'records',
      order: 55,
    })
    app.recordTab('deals.deal', {
      key: 'facts',
      label: 'tab.title',
      icon: Eye,
      permission: 'deals.read',
      order: 900,
      render: FactsTab,
    })
  },
})
```

- The default export is the module and its `key` matches the plugin key.
- `app.routes` adds the page at `/deal-watch` inside the application frame.
- `app.nav` adds the entry to the `records` section. `order: 55` places it among the built-in entries, before the custom object entries at 500. `label` is a translation key, and `permission: 'deals.read'` hides the entry from users who cannot read deals.
- `app.recordTab` adds the tab to `deals.deal` pages with the same permission.

`web/src/WatchPage.tsx`:

```tsx
import {
  Page,
  PageHeader,
  PageLoader,
  RecordListView,
  useCustomObjects,
  useModuleTranslation,
  type FilterNode,
} from '@aphel/web-sdk'

const OPEN_DEALS: FilterNode = {
  op: 'and',
  conditions: [{ field: 'status', operator: 'eq', value: 'open' }],
}

export function WatchPage() {
  const t = useModuleTranslation('deal_watch')
  const objects = useCustomObjects()

  if (objects.isPending) return <PageLoader />

  return (
    <Page>
      <PageHeader title={t('page.title')} description={t('page.description')} />
      <RecordListView recordType="deals.deal" baseFilter={OPEN_DEALS} />
    </Page>
  )
}
```

`RecordListView` needs the record type to be registered. `useCustomObjects()` registers custom object and plugin entity types, so the page waits for it. `baseFilter` keeps only open deals, combined by AND with whatever the user filters. Saved views, columns, sorting and pagination come with the component.

`web/src/FactsTab.tsx`:

```tsx
import {
  PageLoader,
  formatDateTime,
  useModuleTranslation,
  useRecord,
  useRecordFormatting,
  type RecordTabRenderProps,
} from '@aphel/web-sdk'

export function FactsTab({ recordType, recordId }: RecordTabRenderProps) {
  const t = useModuleTranslation('deal_watch')
  const formatting = useRecordFormatting()
  const record = useRecord(recordType, recordId)

  if (!record.data) return <PageLoader />

  return (
    <dl className="deal-watch-facts">
      <dt>{t('tab.created')}</dt>
      <dd>{formatDateTime(record.data.created_at, formatting)}</dd>
      <dt>{t('tab.updated')}</dt>
      <dd>{formatDateTime(record.data.updated_at, formatting)}</dd>
    </dl>
  )
}
```

The tab receives `RecordTabRenderProps`. It loads the record with `useRecord` and formats dates with the organization locale and time zone through `useRecordFormatting`.

`web/src/translations.ts`:

```ts
import type { ModuleTranslations } from '@aphel/web-sdk'

export const translations: ModuleTranslations = {
  'pt-BR': {
    nav: { title: 'Monitor de negócios' },
    page: { title: 'Monitor de negócios', description: 'Negócios em aberto, do mais antigo ao mais recente.' },
    tab: { title: 'Fatos', created: 'Criado em', updated: 'Atualizado em' },
  },
  en: {
    nav: { title: 'Deal watch' },
    page: { title: 'Deal watch', description: 'Open deals, from the oldest to the newest.' },
    tab: { title: 'Facts', created: 'Created at', updated: 'Updated at' },
  },
}
```

`pt-BR` is the fallback language, so it is always present. Keys such as `nav.title` are what `label` refers to.

`web/src/style.css`:

```css
.deal-watch-facts {
  display: grid;
  gap: 0.5rem;
  padding: 1rem;
  border: 1px solid var(--border);
  border-radius: var(--card-radius);
  background: var(--card);
  color: var(--card-foreground);
}

.deal-watch-facts dt {
  color: var(--muted-foreground);
  font-size: 0.75rem;
}
```

The stylesheet reads tokens only, so it follows any interface plugin. It has no reset and no unprefixed utility classes.

Build and install:

```bash
aphel-plugin pack deal-watch -o deal-watch-0.1.0.aphelplugin
curl -s -X POST 'http://127.0.0.1:3001/api/v1/wasm-runtime?acknowledge_web_version=0.1.0' \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -F package=@deal-watch-0.1.0.aphelplugin
```

Reload the page. The entry appears in the navigation for users with `deals.read`.

## Example 3: an interface that replaces the frame and the deal page

A UI plugin with its own colors, a top bar instead of the sidebar, and a different layout for the deal detail page. Other record types keep the default page.

```text
focus-ui/
  plugin.toml
  web/
    package.json
    tsconfig.json
    vite.config.ts
    src/
      index.tsx
      TopbarFrame.tsx
      DealPresentation.tsx
      translations.ts
      style.css
```

`plugin.toml`:

```toml
key = "focus_ui"
name = "Focus interface"
version = "0.1.0"
description = "A top navigation bar and a compact deal page."
api = 1
requires_aphel = ">=0.1.0"

[web]
entry = "web/index.js"
style = "web/style.css"

[ui]
label = "Focus"
description = "Top navigation, softer colors and a compact deal page."

[ui.tokens.shared]
radius = "0.75rem"
font-family-sans = { families = ["Inter", "Segoe UI"], fallback = "sans-serif" }

[ui.tokens.light]
background = "oklch(0.985 0.004 95)"
sidebar = "oklch(0.97 0.006 95)"
primary = "oklch(0.52 0.17 150)"
primary-solid = "oklch(0.52 0.17 150)"
primary-foreground = "#ffffff"
```

The manifest combines `[ui]` tokens (a rounder radius, a warm background, a green primary) with `[web]`. The font token uses two system fonts and a generic fallback, and `primary-foreground` is a hex color. The plugin key is not reserved.

`web/package.json`, `web/tsconfig.json` and `web/vite.config.ts` are the same as in example 2, with the package name `aphel-focus-ui-web`.

`web/src/index.tsx`:

```tsx
import {
  APP_FRAME_CONTRACT,
  RECORD_PRESENTATION_CONTRACT,
  defineModule,
} from '@aphel/web-sdk'
import './style.css'
import { DealPresentation } from './DealPresentation'
import { translations } from './translations'
import { TopbarFrame } from './TopbarFrame'

export default defineModule({
  key: 'focus_ui',
  translations,
  register(app) {
    app.provideComponent(APP_FRAME_CONTRACT, TopbarFrame)
    app.provideComponent(RECORD_PRESENTATION_CONTRACT, DealPresentation)
  },
})
```

Two contract components are provided. They take effect only for users whose interface is this plugin.

`web/src/TopbarFrame.tsx`:

```tsx
import { Search } from 'lucide-react'
import { Link, NavLink } from 'react-router'
import {
  Button,
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  useModuleTranslation,
  useSession,
  type AppFrameProps,
} from '@aphel/web-sdk'

export function TopbarFrame({
  children,
  navigation,
  commands,
  settingsArea,
  returnPath,
  mobileNavOpen,
  onMobileNavOpenChange,
}: AppFrameProps) {
  const t = useModuleTranslation('focus_ui')
  const { me } = useSession()

  return (
    <div className="focus-frame">
      <header className="focus-frame-bar">
        {settingsArea && (
          <Button asChild variant="ghost" size="sm">
            <Link to={returnPath}>{t('frame.back')}</Link>
          </Button>
        )}
        <nav className="focus-frame-nav" aria-label={t('frame.menu')}>
          {navigation.map((entry) => (
            <NavLink key={entry.key} to={entry.to} end={entry.end} className="focus-frame-link">
              <entry.icon size={16} aria-hidden />
              {entry.label}
            </NavLink>
          ))}
        </nav>
        <Button variant="outline" size="sm" onClick={() => commands.openPalette()}>
          <Search aria-hidden />
          {t('frame.search')}
        </Button>
        <span className="focus-frame-user">{me.user.display_name}</span>
      </header>
      <Dialog open={mobileNavOpen} onOpenChange={onMobileNavOpenChange}>
        <DialogContent side="left">
          <DialogHeader>
            <DialogTitle>{t('frame.menu')}</DialogTitle>
          </DialogHeader>
          <nav className="focus-frame-drawer">
            {navigation.map((entry) => (
              <NavLink
                key={entry.key}
                to={entry.to}
                end={entry.end}
                className="focus-frame-link"
                onClick={() => onMobileNavOpenChange(false)}
              >
                <entry.icon size={16} aria-hidden />
                {entry.label}
              </NavLink>
            ))}
          </nav>
        </DialogContent>
      </Dialog>
      <main className="focus-frame-main">{children}</main>
    </div>
  )
}
```

- The frame renders `children`, which is the page (or a loader while the interface resolves).
- The links come from `navigation`, which is already filtered by permission and ordered according to user preferences. Each `entry.icon` is a component.
- `commands.openPalette()` opens the command palette.
- On a settings page `settingsArea` is set, so the bar shows a link to `returnPath`.
- The dialog is the mobile menu. The bottom bar of the shell sets `mobileNavOpen`, and closing the dialog calls `onMobileNavOpenChange(false)`. The CSS hides the horizontal links on narrow screens.

`web/src/DealPresentation.tsx`:

```tsx
import {
  Button,
  RecordFieldsPanel,
  RecordHeader,
  RecordPageShell,
  RecordTabs,
  Slot,
  useModuleTranslation,
  type RecordPresentationProps,
} from '@aphel/web-sdk'

export function DealPresentation({ model, defaultTab, className }: RecordPresentationProps) {
  const t = useModuleTranslation('focus_ui')
  const { title, back, readOnly, edit, record, fields, viewLayout, readOnlyFieldKeys, saveField, tabs, slotProps, actions } = model

  return (
    <RecordPageShell
      className={className}
      header={
        <>
          <RecordHeader
            title={title}
            titleAccessory={<Slot name="record.detail.title" props={slotProps} />}
            back={back}
            primaryAction={
              readOnly ? undefined : (
                <Button size="sm" onClick={edit}>
                  {t('deal.edit')}
                </Button>
              )
            }
            actions={
              <>
                {actions}
                <Slot name="record.detail.actions" props={slotProps} />
              </>
            }
          />
          <Slot name="record.detail.header" props={slotProps} />
        </>
      }
      sidebar={
        <RecordFieldsPanel
          title={t('deal.details')}
          fields={fields}
          layout={viewLayout}
          record={record}
          readOnly={readOnly}
          readOnlyFieldKeys={readOnlyFieldKeys}
          onSaveField={saveField}
        >
          <Slot name="record.fields.panel" props={slotProps} />
        </RecordFieldsPanel>
      }
    >
      <RecordTabs tabs={tabs} defaultTab={defaultTab ?? tabs[0]?.key} />
    </RecordPageShell>
  )
}

DealPresentation.recordTypes = ['deals.deal'] as const
```

- `model` carries everything: the title, the fields, the tabs and the save function. The component builds the page from the exported parts, so inline editing and permissions keep working.
- The `record.detail.*` and `record.fields.panel` slots are rendered so that contributions of other plugins still appear.
- `DealPresentation.recordTypes` limits it to `deals.deal`. Every other record type uses the default presentation.

`web/src/translations.ts`:

```ts
import type { ModuleTranslations } from '@aphel/web-sdk'

export const translations: ModuleTranslations = {
  'pt-BR': {
    frame: { search: 'Buscar', back: 'Voltar', menu: 'Menu' },
    deal: { edit: 'Editar', details: 'Detalhes' },
  },
  en: {
    frame: { search: 'Search', back: 'Back', menu: 'Menu' },
    deal: { edit: 'Edit', details: 'Details' },
  },
}
```

`web/src/style.css`:

```css
.focus-frame {
  display: flex;
  min-width: 0;
  flex: 1;
  flex-direction: column;
}

.focus-frame-bar {
  position: sticky;
  top: 0;
  z-index: 20;
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0 1rem;
  height: 3.5rem;
  border-bottom: 1px solid var(--border);
  background: var(--card);
}

.focus-frame-nav {
  display: flex;
  min-width: 0;
  flex: 1;
  gap: 0.25rem;
  overflow-x: auto;
}

.focus-frame-link {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.75rem;
  border-radius: var(--button-radius);
  color: var(--muted-foreground);
  font-size: 0.875rem;
  white-space: nowrap;
  text-decoration: none;
}

.focus-frame-link:hover {
  background: var(--accent);
  color: var(--accent-foreground);
}

.focus-frame-link[aria-current='page'] {
  background: var(--accent);
  color: var(--foreground);
  font-weight: 600;
}

.focus-frame-main {
  flex: 1;
  padding: 1.5rem;
}

.focus-frame-drawer {
  display: grid;
  gap: 0.25rem;
  padding: 1rem;
}

.focus-frame-user {
  font-size: 0.875rem;
}

@media (max-width: 767px) {
  .focus-frame-nav,
  .focus-frame-user {
    display: none;
  }
}
```

Build, install and select it:

```bash
aphel-plugin pack focus-ui -o focus-ui-0.1.0.aphelplugin
curl -s -X POST 'http://127.0.0.1:3001/api/v1/wasm-runtime?acknowledge_web_version=0.1.0' \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -F package=@focus-ui-0.1.0.aphelplugin
```

Then open Account, Appearance and choose Focus, or set it as the organization default with `PUT /api/v1/organization/ui-plugin`. The page reloads with the new colors, the top bar and the new deal page.

## Related pages

- [Web bundle](/_aphel/docs/plugins/web-bundle/)
- [Screens and contributions](/_aphel/docs/plugins/web-screens/)
- [UI plugins](/_aphel/docs/plugins/ui-plugins/)
- [Packaging](/_aphel/docs/plugins/packaging/)
