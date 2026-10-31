# UI library (`@aphel/ui`)

Aphel ships its base components as a library that plugins can use. **It is a suggestion, not a requirement**: a plugin that builds its own interface keeps working. A plugin that uses the library gets the Aphel look by default and follows the installation's theme without any extra work.

The library is the same code the Aphel web app uses (`web/src/components/ui`). The host serves it to plugins at `/shared/ui.js`, mapped by the import map to the bare specifier `@aphel/ui`, so there is a single copy of React, Radix and the toast state.

## Using it in a plugin bundle

```tsx
import { Button, Card, CardContent, Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@aphel/ui'
```

Add `@aphel/ui` to the externals of the bundler, next to `react` and `@aphel/web-sdk`. For types and the tokens, install the types-only package from `sdk/js/aphel-ui`:

```bash
npm i -D @aphel/ui
```

```css
@import '@aphel/ui/theme.css';
```

`@aphel/ui/theme.css` maps the tokens to Tailwind (`eq:bg-card`, `eq:rounded-lg`, `eq:shadow-md`) for the plugin's own prefixed classes. `@aphel/ui/tokens.css` holds the default token values and is only needed outside Aphel, for example in a standalone preview. Inside Aphel the host defines the tokens.

`@aphel/web-sdk` re-exports everything from `@aphel/ui`, so existing plugins keep working.

A plugin that imports `@aphel/ui` needs a host that serves it: declare `requires_aphel` with the version that introduced it.

## Public web apps (`/p/<key>/`)

The page has no import map and its Content-Security-Policy forbids inline scripts, so the bundle imports the shared modules by absolute URL and links the stylesheet:

```js
export default {
  build: {
    rolldownOptions: {
      external: ['react', 'react/jsx-runtime', 'react-dom/client', '@aphel/ui'],
      output: {
        paths: {
          react: '/shared/react.js',
          'react/jsx-runtime': '/shared/jsx-runtime.js',
          'react-dom/client': '/shared/react-dom-client.js',
          '@aphel/ui': '/shared/ui.js',
        },
      },
    },
  },
}
```

```html
<link rel="stylesheet" href="/shared/ui.css" />
```

Never bundle React into a public app: two copies of React in one page break hooks. `/shared/ui.css` carries Tailwind's preflight, the tokens and the classes the components use.

## Labels

Components never depend on i18n. Texts that belong to the library (close button, retry, confirm dialog, page loader) come from `UiLabelsProvider`; without a provider they follow `document.documentElement.lang` (`en`, otherwise pt-BR). Aphel's web app provides its own translated labels. A plugin can wrap its tree in `UiLabelsProvider` to override any of them.

## Components

| Group | Components |
| --- | --- |
| Actions | `Button`, `DropdownMenu*` |
| Forms | `Input`, `Textarea`, `Select*`, `Checkbox`, `Switch`, `Label`, `Field`, `OptionCard` |
| Layout | `Card*`, `Separator`, `Tabs*`, `Table*`, `PageHeader` |
| Overlays | `Dialog*`, `ConfirmDialog`, `Popover*`, `Tooltip*`, `TooltipProvider` |
| Feedback | `Badge`, `Avatar`, `EmptyState`, `ErrorState`, `Skeleton`, `Spinner`, `PageLoader`, `Toaster`, `toast` |
| Utilities | `cn`, `UiLabelsProvider`, `useUiLabels`, `defaultUiLabels` |

`Select` is Radix's: an item cannot have the empty string as its value, so use a sentinel value for a "none" option.

## Tokens

Tokens are CSS custom properties on `:root` (and `[data-theme='dark']`). They are generated from `back/modules/themes/tokens.json` into `web/src/sdk/tokens.css` by `make modules`; the full list with defaults is in [`tokens.md`](tokens.md). Components read them, so changing a token changes every component that uses the library, in Aphel and in plugins. An installation changes them with a [theme](../themes/README.md).

| Group | Tokens |
| --- | --- |
| Color | `--background`, `--foreground`, `--card`, `--popover`, `--primary`, `--primary-solid`, `--primary-foreground`, `--secondary`, `--muted`, `--accent`, `--destructive`, `--success`, `--warning` (each with `-foreground` where applicable), `--border`, `--input`, `--ring`, `--sidebar*` |
| Shape | `--radius`, `--button-radius`, `--input-radius`, `--checkbox-radius`, `--badge-radius`, `--card-radius`, `--card-border-width`, `--dialog-radius`, `--popover-radius` |
| Size | `--control-height-sm`, `--control-height-md`, `--control-height-lg` |
| Depth | `--elevation-xs`, `--elevation-sm`, `--elevation-md`, `--elevation-lg`, `--card-shadow`, `--popover-shadow`, `--overlay` |
| Type | `--font-family-sans`, `--font-family-heading`, `--font-family-mono` |
| Motion | `--duration-fast` |

The organization color overrides `--primary*` through `--org-primary*`, on top of the theme.

A public web app of a plugin (`/p/<key>/`) also links the active theme's stylesheet:

```html
<link rel="stylesheet" href="/api/v1/public/themes/active.css" />
```

## Stability

The exports of `@aphel/ui` and the token names only grow. Removing or renaming either is a breaking change that follows [module compatibility](../modules/compatibility.md). Default token values may be refined between releases.
