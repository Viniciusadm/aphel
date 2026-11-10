---
title: UI plugins
description: Build an interface plugin that changes the colors of Aphel, replaces the application frame or redesigns the record page.
sidebar:
  order: 52
---

An interface plugin (a UI plugin) is a plugin that declares `[ui]` in `plugin.toml`. Every user can choose it as their interface, and administrators can make it the default for the organization. It can do two things:

- Change design tokens: colors, radius, shadows, fonts and sizes. This needs only `plugin.toml`, no code.
- Replace whole screens by providing two contract components from a web bundle: the application frame (`app.Frame`) and the record detail presentation (`records.Presentation`).

A UI plugin can also be an ordinary plugin at the same time: it may have a backend, entities and any other contribution. Its bundle is loaded only while the plugin is the selected interface.

## The `[ui]` table

```toml
[ui]
label = "Midnight"
description = "Slate surfaces and a teal accent, with sharper corners."

[ui.tokens.shared]
radius = "0.375rem"

[ui.tokens.light]
primary = "#0f766e"

[ui.tokens.dark]
primary = "#2dd4bf"
```

| Key | Required | Rule |
| --- | --- | --- |
| `label` | Yes | 1 to 60 characters after trimming. Shown on the Appearance page. |
| `description` | No | Up to 300 characters. |
| `tokens.shared` | No | Tokens applied in light and dark mode. |
| `tokens.light` | No | Tokens applied when the dark theme is not active. |
| `tokens.dark` | No | Tokens applied when the dark theme is active. |

Validation of the whole table:

- Unknown keys are rejected, in `[ui]` and in `[ui.tokens]` (only `shared`, `light` and `dark` exist). There is no `fonts` key, so a plugin cannot bundle fonts.
- At most 400 tokens in total across the three sets.
- The plugin key cannot be `core` or `default` (error on `key`: "is reserved for interface plugins"). Those names are used for the original interface and the organization default stylesheet.
- Every violation is reported with its path, for example `ui.tokens.light.background`, under the error code `plugin_manifest_invalid`.

### Token values

A token name is the name of a CSS custom property without the leading dashes. The full list, with kinds and defaults, is in [Design tokens](/_aphel/docs/reference/design-tokens/). A name that is not in the list is rejected with "is not a design token". Every value must be a string, except fonts. Values are trimmed and must not be empty.

| Kind | Accepted values |
| --- | --- |
| `color` | `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`; `rgb()`, `rgba()`, `hsl()`, `hsla()`, `oklch()`, `oklab()` with digits, letters, `.`, `%`, `+`, `-`, `/`, `,` and spaces inside (up to 80 characters); `transparent`; `currentColor`. Named colors such as `red` are rejected. |
| `length` | `0` or a number with `px`, `rem`, `em` or `%` (negative allowed); or `calc(var(--radius) + 4px)` and `calc(var(--radius) - 2px)` with `px`, `rem` or `em`. |
| `duration` | A number with `ms` or `s`, such as `150ms` or `0.2s`. |
| `shadow` | `none`, or a comma separated list of layers `[inset] x y [blur [spread]] color`. Offsets and sizes use `px` or `rem` (or `0`). The color follows the color rules, so `rgb(0 0 0 / 0.1)` works. |
| `font` | An inline table `{ families = [...], fallback = "..." }`, or a reference to another font token. |

Any non-font token can instead reference another token of the same kind with exactly `var(--name)`: for example `ring = "var(--primary)"`. A reference to a token of a different kind, to an unknown token, or with a fallback value inside `var()` is rejected.

Font tokens (`font-family-sans`, `font-family-heading`, `font-family-mono`) take:

```toml
font-family-sans = { families = ["Inter", "Segoe UI"], fallback = "sans-serif" }
```

- `families`: 1 to 6 names. Each name uses letters, digits, spaces, `_` and `-` (up to 64 characters), and must be one of the system fonts the host accepts: Inter, Arial, Helvetica, Georgia, Times New Roman, Courier New, Verdana, Tahoma, Trebuchet MS, Segoe UI, Roboto, Menlo, Consolas, Monaco, SF Pro Text, Helvetica Neue, Noto Sans. Other names fail with "is neither bundled in `fonts` nor a known system font".
- `fallback`: one of `sans-serif`, `serif`, `monospace`, `system-ui`, `ui-sans-serif`, `ui-serif`, `ui-monospace`.

Rules for every value:

- At most 300 characters.
- It cannot contain any of these fragments, in any letter case: `;` `{` `}` `\` `"` `'` `<` `>` `@` `/*` `url(` `expression`. Images and `@import` are therefore impossible.

Examples of error messages: `must be a hex, rgb(), hsl(), oklch() or oklab() color`, `must be a length such as 4px, 0.5rem or calc(var(--radius) - 2px)`, `must be a duration such as 150ms or 0.2s`, `must be a list of shadows: [inset] x y [blur [spread]] color`, `--radius is not a token of the same kind`.

### How tokens become CSS

The server renders the sets into a stylesheet:

```css
html:root{--radius:0.375rem;}
html:root:not([data-theme='dark']){--primary:var(--org-primary, #0f766e);}
html[data-theme='dark']{--primary:var(--org-primary-dark, #2dd4bf);}
```

Shared tokens go to `html:root`, light tokens to `html:root:not([data-theme='dark'])` and dark tokens to `html[data-theme='dark']`. These selectors have higher specificity than the defaults of the application.

The tokens `primary`, `primary-solid` and `primary-foreground` are wrapped so that the brand color of the organization still wins when an administrator has set one (`--org-primary`, `--org-primary-solid`, `--org-primary-foreground`, and `--org-primary-dark` for the dark `primary`). See [UI library](/_aphel/docs/plugins/ui-library/).

:::note
The application currently always runs in the light theme: the `data-theme` attribute is set to `light` and there is no switch. Tokens in `[ui.tokens.dark]` are validated and served, but they have no visible effect until a dark theme is enabled. Use `shared` and `light` for what users see today.
:::

## A complete colors-only plugin

`plugin.toml` is the only file:

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

Pack it with `aphel-plugin pack` (see [Packaging](/_aphel/docs/plugins/packaging/)). It has no `[web]`, so it contains no code and the consent dialog shows no code warning. The same file is walked through in [Examples](/_aphel/docs/plugins/examples/).

## Replaceable screens

With a `web/` bundle, a UI plugin provides components for two contracts through `app.provideComponent`. Only the plugin that is the effective interface is consulted for these contracts, and it must provide at most one component per contract (two providers from the same plugin for the same contract count as none).

### `app.Frame`

The frame is everything around the page: sidebar or top bar, user menu, mobile navigation. Register it with the exported constant:

```tsx
import { APP_FRAME_CONTRACT, defineModule } from '@aphel/web-sdk'

export default defineModule({
  key: 'focus_ui',
  register(app) {
    app.provideComponent(APP_FRAME_CONTRACT, TopbarFrame)
  },
})
```

The component receives `AppFrameProps`:

| Prop | Type | Meaning |
| --- | --- | --- |
| `children` | `ReactNode` | The page. A loading indicator is passed while the interface is still resolving. You must render it. |
| `navigation` | `NavigationEntry[]` | Visible entries, already filtered by permission, user order and hidden items. See below. |
| `commands` | `CommandPaletteControls` | `openPalette(mode?)`, `closePalette()`, `togglePalette()`, `openCreate(request)`, `openHelp()`, `registerCommands(owner, commands)`, `unregisterCommands(owner)`. |
| `settingsArea` | `SettingsArea \| null` | Set while the user is on a settings page: `{ key, basePath, titleKey, descriptionKey, icon, scope, primary }`. |
| `returnPath` | `string` | The last path outside settings, for a "back" link. |
| `mobileNavOpen` | `boolean` | Whether the mobile menu was requested. |
| `onMobileNavOpenChange` | `(open: boolean) => void` | Report that the mobile menu opened or closed. |

`NavigationEntry` has `key`, `module`, `to`, `label` (already translated), `icon` (a `LucideIcon` component), `order`, `end?`, `locked?` (the Settings entry), `badge?`, `group?: { key, label, icon }`, `section?` and `mobile?`.

Things the default frame does that yours must decide about:

- On screens below 768 px wide the shell shows a bottom bar with a "More" button. That button sets `mobileNavOpen` to true. If your frame ignores it, the button does nothing, so render a menu when it is true and call `onMobileNavOpenChange(false)` when it closes.
- When `settingsArea` is not null, the default frame replaces the main navigation with the navigation of that settings area. If your frame does not render settings navigation, users still reach the pages from the overview at `settingsArea.basePath`. A back link to `returnPath` is a good minimum.
- The default frame renders the contributions to the slots `app.sidebar`, `app.topbar` and `user.menu`. Render `<Slot name="app.topbar" props={{}} />` and the others if you want other plugins' contributions to keep working. The `app.shell` slot is rendered by the host outside the frame.
- The Settings entry has `locked: true` and key `settings`.

If your frame throws while rendering, the host shows the default frame with the same props.

### `records.Presentation`

The presentation draws the record detail page: header, fields, tabs. The host loads the record and builds a model, then hands both to the presentation.

```tsx
import { RECORD_PRESENTATION_CONTRACT, defineModule } from '@aphel/web-sdk'

export default defineModule({
  key: 'focus_ui',
  register(app) {
    app.provideComponent(RECORD_PRESENTATION_CONTRACT, DealPresentation)
  },
})
```

Props (`RecordPresentationProps`): `model: RecordPageModel`, `defaultTab?: string`, `className?: string`.

`RecordPageModel` fields:

| Field | Meaning |
| --- | --- |
| `recordType`, `recordId`, `record` | The record. |
| `schema` | The field schema of the type. |
| `title` | Display title. |
| `subtitle` | `{ field, value } \| null`, the subtitle field of the type. |
| `owner` | `{ field, value }` for the owner field. |
| `fields` | Active fields of the type. |
| `viewLayout` | The saved view layout, or null. |
| `visibleFields` | Fields visible according to the layout and rules. |
| `summaryFieldKeys` | Keys of the summary fields of the type. |
| `readOnly`, `readOnlyFieldKeys` | Editing state. |
| `back` | Back link `{ to, label? }` or undefined. |
| `edit()` | Navigates to the edit page. |
| `saveField(field, value)`, `isSaving` | Inline save. |
| `tabs` | Resolved tabs (default and module tabs, after the `records.detailTabs` filter, permissions and module activity). Each is `{ key, label, content, summary?, icon?, group?, source?, permission? }`. |
| `slotProps` | The props for the record slots: `{ recordType, record, readOnly }`. |
| `sectionContext` | `{ recordType, recordId, readOnly }`. |
| `actions` | Header actions passed by the caller. |

Rules:

- To apply to only some record types, set a static `recordTypes` property on the component: `DealPresentation.recordTypes = ['deals.deal']`. For other record types the default presentation is used. The type `RecordPresentationComponent` describes this.
- The default presentation renders the `record.detail.*` and `record.fields.panel` slots. Render those slots in your presentation as well, otherwise contributions of other plugins disappear from the page.
- Build the page with the exported parts: `RecordPageShell`, `RecordHeader`, `RecordFieldsPanel`, `RecordTabs`, `Slot`. They keep inline editing, permissions and tab navigation working.
- If your presentation throws, the host shows an alert and renders the default presentation instead, and keeps using the default presentation for that page view.

A complete frame and presentation are in [Examples](/_aphel/docs/plugins/examples/).

## How users and administrators choose an interface

| Where | What it does |
| --- | --- |
| Account, Appearance (`/account/appearance`) | Lists every enabled interface plugin with color swatches (the `primary`, `background`, `foreground` and `accent` tokens when set), plus "Follow the organization default" and "Aphel original". Plugins with a bundle are marked as running code. Saving reloads the page. |
| Organization default (same page, needs `settings.manage`) | Sets the interface used by everyone who has not chosen one and by the sign-in page. |
| `GET /api/v1/ui-plugins` | Lists enabled interface plugins: `key`, `label`, `description`, `has_screens` (has a `[web]` bundle), `has_colors` (declares tokens) and `preview`. Any signed-in user. |
| `GET /api/v1/preferences` | Returns `ui_plugin` (the saved choice, null follows the default), `ui_plugin_default` and `effective_ui_plugin` (what applies now, null for the original interface). |
| `PATCH /api/v1/preferences` with `{ "ui_plugin": "midnight" }` | Saves the personal choice. Use `"core"` for the original interface and `null` to follow the organization. A key that is not an enabled interface plugin fails with a validation error on `ui_plugin`. |
| `PUT /api/v1/organization/ui-plugin` with `{ "ui_plugin": "midnight" }` | Sets the organization default. Needs `settings.manage`. `null` selects the original interface. `core` is not accepted here. A key that is not an enabled interface plugin returns 422. It publishes `organization.ui_plugin_updated` and an audit entry. |

The effective interface is the personal choice if it is an enabled interface plugin, otherwise the organization default if that is enabled, otherwise the original interface. The personal value `core` always means the original interface.

## Loading rules

1. `index.html` links `/api/v1/public/ui/default.css` and, before the app starts, switches the link to `/api/v1/public/ui/<choice>.css` when `localStorage` has the key `aphel.ui`. The stored value is a copy of the saved preference that avoids a flash of the wrong colors.
2. `loadWebPlugins` requests `/api/v1/public/wasm-runtime?ui=<choice>`. The server lists every enabled plugin with a bundle, except interface plugins other than the requested one. `ui=core` asks for no interface plugin. If the requested key is not an enabled interface plugin, the organization default is used instead. The response header `x-aphel-ui-plugin` names the interface plugin that applies, if any.
3. After the user preferences load, the app saves the choice in `localStorage`, points the stylesheet link at the saved preference and compares the effective interface with the one named by that header. If they differ it reloads the page, at most once per browser tab session for the same target.
4. The stylesheet endpoint is public (it is needed before sign-in):

| URL | Response |
| --- | --- |
| `/api/v1/public/ui/<key>.css` | The CSS of that enabled interface plugin. 404 if it does not exist, is disabled or has no `[ui]`. |
| `/api/v1/public/ui/default.css` | The CSS of the organization default, or an empty stylesheet. |
| `/api/v1/public/ui/core.css` | An empty stylesheet. |

Responses are sent with `Cache-Control: no-cache` and an `ETag` based on the package checksum.

## Fallbacks and lifecycle

- If the chosen plugin is disabled, the user falls back to the organization default, then to the original interface. The saved choice is not deleted by disabling, so enabling the plugin again restores it.
- Uninstalling the plugin clears every personal choice that pointed to it and the organization default, in the same transaction.
- If the bundle fails to load, or the plugin has no bundle, the colors still apply from the stylesheet and the screens fall back to the defaults.
- If a contract is provided twice by the same plugin, or `recordTypes` does not include the current type, the default component is used.
- Errors inside a replaced screen fall back to the default one as described above.
- Because the interface is decided at load time, switching always reloads the application.

## Trust

A UI plugin without `[web]` contains only validated token values. The server rejects anything that could inject CSS: no `url()`, no `@import`, no braces, no semicolons. It is safe to install without reviewing code, and the consent dialog shows no code warning.

A UI plugin with `[web]` is application code with the session of every user who selects it, the same as any web bundle. It needs the same acknowledgement (`acknowledge_web_version`) on install and enable, and the Appearance page warns users that it "runs code with your session". See [Security model](/_aphel/docs/plugins/security-model/).

## Related pages

- [Manifest](/_aphel/docs/plugins/manifest/)
- [UI library](/_aphel/docs/plugins/ui-library/)
- [Design tokens](/_aphel/docs/reference/design-tokens/)
- [Examples](/_aphel/docs/plugins/examples/)
