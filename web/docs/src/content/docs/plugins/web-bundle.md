---
title: Web bundle
description: How to build, package and load the browser side of a plugin, including the shared modules, translations and CSS rules.
sidebar:
  order: 50
---

The web side of a plugin is an ES module that the Aphel web app imports while it starts. The module registers screens, navigation entries, record tabs and other contributions through a small API. This page covers everything around that module: the package layout, what the host shares with your code, how and when the bundle loads, translations and CSS. The registration API itself is documented in [Screens and contributions](/docs/plugins/web-screens/).

:::caution
A web bundle runs inside the page with the session of every signed-in user who opens it. The backend capabilities of the plugin (`requires`) do not limit it. Administrators must acknowledge each version before it can be installed or enabled. See [Security model](/docs/plugins/security-model/).
:::

## Package layout

A plugin that has a browser side declares `[web]` in `plugin.toml`:

```toml
[web]
entry = "web/index.js"
style = "web/style.css"
public = "web/app"
```

| Key | Default | Meaning |
| --- | --- | --- |
| `entry` | `web/index.js` | The ES module that default-exports `defineModule(...)`. |
| `style` | none | An optional stylesheet linked into the page before the entry is imported. |
| `public` | none | A directory with a static web app served at `/p/<key>/`. See [Public web apps](/docs/plugins/public-web-apps/). |

Rules that the installer and the server enforce:

- The entry file must exist in the package, otherwise the install fails with `plugin_package_invalid` ("the web entry ... is missing").
- Web files are served only from paths that start with `web/`. Keep `entry`, `style` and every chunk, image and font under `web/`.
- Every file below `web/` is served without authentication at `/api/v1/public/wasm-runtime/<key>/<path>`, but only while the plugin is enabled. Never put secrets in the bundle.
- Assets are sent with `Cache-Control: public, max-age=31536000, immutable`. The entry and style URLs carry a `?v=` parameter derived from the package checksum, so a new package always loads a new entry. Other files are cached for a year under their own URL. Give every chunk, image and font a content hash in its file name (Vite does this by default), otherwise browsers can keep serving the old file after an upgrade.
- File types are detected by extension. `.js`, `.mjs`, `.css`, `.json`, `.wasm`, `.html`, `.svg`, `.png`, `.jpg`, `.jpeg`, `.webp`, `.ico`, `.txt` and `.woff2` get a proper content type. Anything else is sent as `application/octet-stream`.

The `aphel-plugin` CLI packs the contents of `web/dist/` of your project as `web/` inside the package. A project therefore keeps its build output in `web/dist/`, and `entry = "web/index.js"` corresponds to `web/dist/index.js` on disk. See [Packaging](/docs/plugins/packaging/).

## The module contract

The entry must default-export the value returned by `defineModule`, and its `key` must be the plugin key from `plugin.toml`.

```ts
import { defineModule } from '@aphel/web-sdk'

export default defineModule({
  key: 'deal_watch',
  translations: {},
  register(app) {},
})
```

| Property | Type | Meaning |
| --- | --- | --- |
| `key` | `string` | Required. Must equal the plugin key. Also the name of your default translation namespace. |
| `translations` | `Record<locale, resources \| loader>` | Optional. Resources installed in the namespace named after `key`. |
| `namespaces` | `Record<namespace, Record<locale, resources \| loader>>` | Optional. Additional namespaces. |
| `register` | `(app: ModuleApp) => void` | Optional. Called once at startup with the registration API. |

If the default export is missing or its `key` differs from the plugin key, the bundle is rejected with the error `plugin <key> does not export its module as default` and counts as failed.

Do not do work that can throw inside `register`. The host calls `register` for every module in a loop without isolating errors, so an exception thrown there stops the application from starting. Declare contributions only, and do the real work inside the components you register.

### Module key rules

The plugin key is the module key, the translation namespace, the permission prefix and the URL segment of plugin routes. The manifest validator requires 1 to 32 characters: lowercase letters, digits and underscores, starting with a letter. `custom` is reserved, a key that matches a built-in module is rejected, and an interface plugin cannot use `default` or `core`.

## What the host shares with your bundle

The page contains an import map. Your bundle must not ship its own copy of these packages, otherwise React contexts, the query cache and the router are duplicated and nothing works. Mark all of them as external in your build.

| Specifier | What it is |
| --- | --- |
| `react` | The host React. |
| `react/jsx-runtime` | The JSX runtime, needed by the automatic JSX transform. |
| `react-dom` | The host ReactDOM. |
| `react-dom/client` | `createRoot` and `hydrateRoot`. |
| `react-router` | React Router 7. Use it for `Link`, `NavLink`, `useNavigate`, `useParams` and route objects. |
| `@tanstack/react-query` | TanStack Query. Your queries use the query client of the host. |
| `react-i18next` | The i18n bindings, bound to the instance of the host. |
| `@aphel/web-sdk` | The public SDK: `defineModule`, record building blocks, API client, hooks and the UI library. |
| `@aphel/ui` | Only the UI library, also re-exported by `@aphel/web-sdk`. |

Everything else is yours to bundle, including `lucide-react`. The host does not provide an icon package to plugins, and navigation entries and tabs take a `LucideIcon` component, so import icons from `lucide-react` (named imports are tree-shaken) or pass any component that accepts `className` and `size`.

In development builds of Aphel the import map points at the source files of the web app. In production builds it points at stable files served under `/shared/`. Your code imports the same specifiers in both cases.

### What `@aphel/web-sdk` exports

| Area | Main exports |
| --- | --- |
| Module API | `defineModule`, `NAV_ORDER`, `Slot`, `ContractComponent`, `useContractComponent`, `useHasContractComponent`, `APP_FRAME_CONTRACT`, `RECORD_PRESENTATION_CONTRACT`, `useModuleTranslation`, `useChangeFeed`, and the types `ModuleApp`, `WebModule`, `NavContribution`, `NavGroup`, `RecordTabContribution`, `RecordSectionContribution`, `RecordTabRenderProps`, `SettingsContribution`, `SettingsAreaContribution`, `SettingsGroupContribution`, `SlotContribution`, `SlotName`, `SlotProps`, `SlotPropsMap`, `RecordSlotProps`, `CommandContribution`, `TraitListViewContribution`, `ModuleTranslations`, `AppFrameProps`, `RecordPresentationProps`, `RecordPresentationComponent` |
| HTTP and session | `api`, `apiRequest`, `ApiError`, `isApiError`, `useSession` |
| Feedback | `ConfirmDialog`, `ErrorBoundary`, `PageLoader`, `errorMessage`, `notifyError`, `notifySuccess` |
| Records | `RecordListView`, `SearchBox`, `RecordDetailPage`, `useRecord`, `useUpdateRecord`, `RecordForm`, `RecordFormDialog`, `KanbanBoard`, `MonthGrid`, `AgendaList`, `Timeline`, `GenericTimelineContent`, `FilterBuilder`, `FilterChips`, `FilterMenu`, `recordsApi`, `recordsKeys`, `useFieldSchema`, `useOrganizationUsers`, `recordDetailHref`, `recordListPath`, `viewHref`, `recordTypeIcon`, `registerRecordResource`, `getRecordResource`, `formatCurrency`, `formatDate`, `formatDateTime`, `formatNumber`, `useRecordFormatting`, `FieldValueDisplay`, `UserValue`, `ShareRecordButton` and the record types `RecordItem`, `FieldDefinition`, `FieldSchema`, `FilterNode`, `FilterCondition`, `SavedView`, `RecordTab` |
| Record page building blocks | `useRecordPageModel`, `useRecordDetailData`, `DefaultRecordPresentation`, `RecordHeader`, `RecordFieldsPanel`, `RecordTabs`, `RecordPageShell`, `RECORD_TAB_PARAM`, `useActiveTab` |
| Custom objects and entities | `useCustomObjects`, `useRelationships`, `ObjectIcon`, `customObjectPath`, `customRecordPath`, `customRecordType`, `isCustomRecordType`, `useEntities`, `useEntityLabel`, `useEntityHasTrait` |
| Aggregates | `useAggregate`, `useAggregateSchema`, `aggregatesApi`, `aggregatesKeys` |
| Saved views | `useSavedViews`, `ViewSwitcher`, `useListUrlState` and related list-state helpers |
| Domain areas | Typed clients and hooks for approvals, calendar helpers, configuration packages, dashboard, email, e-signature, formulas, imports, money and currencies, people, pricing, proposals, segments, SSO, tasks and activities, timeline, line items and workflow pipelines |
| Settings helpers | `useRoles`, `useTeams`, `useAllMembers`, `useFields`, `useOrganizationConfiguration`, `RequirePermission`, `NoAccess`, `SortableList`, `FormRow`, `ChoiceSelect`, `FormField` |
| Files | `AttachmentsPanel`, `uploadFile`, `fileDownloadUrl`, `useAttachments` |
| Command palette | `useRegisterCommands`, `PaletteCommand` |
| UI library | Everything in `@aphel/ui`. See [UI library](/docs/plugins/ui-library/). |

## When and how bundles load

1. The browser loads the shell and the import map.
2. Before it builds the router, the app calls `GET /api/v1/public/wasm-runtime`. The response lists, for each enabled plugin that declares `[web]`: `key`, `name`, `version`, `menu`, `role_homes`, `ui` (true for interface plugins), `entry` and `style`.
3. For each listed plugin, the app appends a `<link rel="stylesheet">` for `style` and imports `entry`. All bundles load in parallel and a failure in one does not stop the others.
4. After the imports settle, the app registers the built-in modules first and then the plugin modules, calling `register` for each. Contributions with the same `order` keep that registration order.
5. The router, the navigation and the translations are built from those registrations.

Consequences:

- A plugin takes effect only after a page reload. The Plugins page reloads the application a moment after an install, upgrade, enable, disable or removal. When you install through the API you reload the page yourself.
- A bundle that fails to load (network error, syntax error, missing default export, key mismatch) is logged with `console.error` and the user sees a notice titled "Part of the system did not load" naming the plugin. The rest of the application keeps working.
- Interface plugins (`[ui]`) are listed only when they are the one chosen for the current user or the organization default. See [UI plugins](/docs/plugins/ui-plugins/).
- If the list request itself does not succeed, no plugin bundle loads.
- A plugin module is active as soon as the module list of the installation has loaded, and disabled plugins are never listed, so enablement is controlled entirely by enabling or disabling the plugin.

## Translations

User-facing text lives in translation resources. Declare them on the module:

```ts
import { defineModule } from '@aphel/web-sdk'

export default defineModule({
  key: 'deal_watch',
  translations: {
    'pt-BR': {
      nav: { title: 'Monitor de negócios' },
      errors: { deal_watch_limit: 'Limite atingido.' },
    },
    en: {
      nav: { title: 'Deal watch' },
      errors: { deal_watch_limit: 'Limit reached.' },
    },
  },
})
```

Rules:

- `translations` is installed in the namespace named after the module `key`. `namespaces` installs extra namespaces. A namespace shared by several modules is removed only when none of its owners is active.
- A locale value is either the resources object or a loader `() => Promise<resources | { default: resources }>`, so large catalogs can be loaded on demand: `en: () => import('./en.json')`.
- The application loads the resources for the current language and for `pt-BR`, which is the fallback language. Always provide `pt-BR`. The shipped languages are `pt-BR` and `en`. Any other locale you declare is added to the language choices.
- Labels that you pass to the registration API (`label`, `description`, `group.label`, the settings titles) are translation keys looked up in your namespace. If a key is not found the key itself is displayed, so plain text also works when you do not translate. A key written as `namespace:key` is looked up in that namespace instead.
- In components, call `useModuleTranslation('deal_watch')` to get a `t(key, options)` function bound to your namespace.
- API error codes are translated by `errors.<code>` in your namespace. For a code such as `deal_watch_limit`, the host looks for `errors.deal_watch_limit` in the namespace formed by the code prefixes (`deal_watch`, then `deal`) and shows the text in the error toast, and `errorMessage(error)` returns it.
- Widget titles are looked up at `widgets.<widget key>.title` in your namespace and fall back to the title in the manifest.

## A bundle without a build step

The import map lets a plain JavaScript module use the shared packages directly. Place the file in `web/dist/index.js` of the project so that the CLI packs it as `web/index.js`.

`plugin.toml`:

```toml
key = "hello_web"
name = "Hello web"
version = "0.1.0"
api = 1

[web]
entry = "web/index.js"
```

`web/dist/index.js`:

```js
import { defineModule } from '@aphel/web-sdk'
import { createElement } from 'react'

function HelloIcon(props) {
  return createElement(
    'svg',
    {
      viewBox: '0 0 24 24',
      fill: 'none',
      stroke: 'currentColor',
      strokeWidth: 2,
      width: props.size ?? 16,
      height: props.size ?? 16,
      className: props.className,
      'aria-hidden': true,
    },
    createElement('circle', { cx: 12, cy: 12, r: 9 }),
  )
}

function HelloPage() {
  return createElement('p', null, 'Hello from a plugin without a build step.')
}

export default defineModule({
  key: 'hello_web',
  register(app) {
    app.routes([{ path: 'hello', element: createElement(HelloPage) }])
    app.nav({ to: '/hello', label: 'Hello', icon: HelloIcon })
  },
})
```

Run `aphel-plugin pack` in the project directory to produce `hello_web-0.1.0.aphelplugin`. The page appears at `/hello` and in the navigation after the plugin is installed and the page is reloaded.

## Vite and TypeScript

A typical project keeps the web code in `web/` next to `plugin.toml`. The CLI runs `npm run build` inside `web/` when `web/package.json` defines a `build` script, and packs `web/dist/`.

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

The output is `dist/index.js` and, when you import CSS, `dist/style.css`, matching `entry = "web/index.js"` and `style = "web/style.css"`.

The types of `@aphel/web-sdk` come from the Aphel source tree. Point the compiler at a checkout of Aphel with path mappings. The SDK sources import each other through the `@/` alias, so map that alias to the web sources as well.

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
      "@aphel/ui": ["../../aphel/web/src/sdk/ui.ts"],
      "@/*": ["../../aphel/web/src/*"]
    }
  },
  "include": ["src"]
}
```

Adjust the relative paths to where your Aphel checkout is. The checkout also needs its own `node_modules` installed (`npm install` in `web/` of Aphel), because the SDK sources import React, TanStack Query and Radix. Alternatively, run `npm run generate` in `sdk/js/aphel-ui` to produce declaration files for `@aphel/ui` only and map `@aphel/ui` to `sdk/js/aphel-ui/dist/sdk/ui.d.ts`.

Type checking and bundling are separate. Vite does not type check, so run `npm run typecheck` before packing.

## CSS rules

Plugin CSS is added to the page after the CSS of the application. Follow these rules so it neither breaks nor is broken by the host:

- Prefer the design tokens (`var(--border)`, `var(--card)`, `var(--primary)`, `var(--button-radius)` and the rest listed in [Design tokens](/docs/reference/design-tokens/)) over fixed colors, so your screens follow the chosen interface and the organization brand color.
- Never import Tailwind's preflight or any reset. The host already applies one, and a second reset overrides the host styles. Unlayered CSS also beats every layered rule of the host.
- If you use Tailwind, give your utilities a prefix, for example `eq`, and write the classes as `eq:flex`, `eq:p-4`, `eq:dark:bg-card` (the prefix always comes first). Without a prefix your utilities collide with the utilities of the host, which compiles only the classes it uses.
- The layer order of the page is `theme, base, components, plugin-ui, utilities`. Put your theme in `theme` and your utilities in `utilities`.
- Import the token mapping from `@aphel/ui/theme.css` so utilities such as `bg-card` and `text-muted-foreground` resolve to the tokens. It contains only `@theme inline` mappings and the dark variant, so it adds no rules of its own.

`src/style.css`:

```css
@layer theme, base, components, plugin-ui, utilities;

@import 'tailwindcss/theme.css' layer(theme) prefix(eq);
@import 'tailwindcss/utilities.css' layer(utilities) prefix(eq);
@import '@aphel/ui/theme.css';

@source '.';
```

With the Tailwind plugin for Vite (`@tailwindcss/vite`) added to `plugins` in `vite.config.ts`, a class such as `eq:rounded-lg eq:bg-card eq:p-4` compiles to rules inside `@layer utilities` that read `var(--radius)`, `var(--card)` and a spacing variable named `--eq-spacing`. The bundle then contains only your prefixed utilities and no reset.

`@aphel/ui` can be installed from the Aphel checkout as a local dependency, for example `"@aphel/ui": "file:../../aphel/sdk/js/aphel-ui"`, after running `npm run generate` there.

## Related pages

- [Screens and contributions](/docs/plugins/web-screens/) for the `app` registration API.
- [UI plugins](/docs/plugins/ui-plugins/) for plugins that change colors or replace whole screens.
- [Packaging](/docs/plugins/packaging/) for `aphel-plugin` and installation.
- [Testing and debugging](/docs/plugins/testing-and-debugging/) for diagnosing a bundle that does not load.
