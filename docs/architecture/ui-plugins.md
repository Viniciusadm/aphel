# UI plugins

There is one extension mechanism: the plugin. A plugin that also changes how Aphel looks declares an interface in its manifest and becomes a **UI plugin**. Every user picks at most one UI plugin; everything it does not provide stays the original Aphel.

## What a UI plugin can do

```toml
key = "midnight"
name = "Midnight"
version = "1.0.0"
api = 1

[ui]
label = "Midnight"
description = "Dark blue palette."

[ui.tokens.light]
background = "#0f172a"
foreground = "#e2e8f0"
```

| Declaration | Effect | Needs code consent |
| --- | --- | --- |
| `[ui.tokens.shared]`, `[ui.tokens.light]`, `[ui.tokens.dark]` | Design tokens (colors, lengths, shadows, durations, fonts) served as CSS | No: data validated per token kind (see [tokens](../ui/tokens.md)) |
| `[web]` bundle calling `app.provideComponent(contract, Component)` | Replaces a screen | Yes: `acknowledge_web_version`, as for any web code |

A plugin without `[ui]` is an ordinary plugin and is not offered to users. The keys `default` and `core` are reserved.

Tokens are checked at install: unknown tokens, values with `;{}"'<>@/*`, `url(`, `expression` or more than 300 characters, and values of the wrong kind are rejected. Bundled fonts are not supported; use the known system families. The organization brand color (`--org-primary*`) still wins over the plugin's primary tokens.

## Screens

A screen is a contract component, provided with `app.provideComponent` (see `web/src/kernel/modules/contract-components.ts`). Today the replaceable contracts selected by the UI plugin are:

| Contract | Replaces | Receives |
| --- | --- | --- |
| `app.Frame` | The application frame: navigation, top bar, content area | `AppFrameProps` |
| `records.Presentation` | The record detail page | `RecordPresentationProps` (the page model: fields, tabs with their `source`, actions, `saveField`) |

A presentation may declare `Component.recordTypes = ['deals.deal']`; for other record types the standard detail is used. Everything else the plugin does not provide uses the Aphel component. If the selected plugin is disabled, uninstalled or fails to render, the page falls back to the standard component and the saved choice is kept.

Validation, permissions and saving stay on the server and in the shared page model, so a screen cannot relax them.

## Selection

- **Organization default:** `organization_settings.settings.$.ui_plugin`, set with `PUT /api/v1/organization/ui-plugin` (`settings.manage`) or in Account › Appearance. It applies to people who have not chosen and to the sign-in page.
- **Personal choice:** `user_preferences.ui_plugin` through `PATCH /api/v1/preferences` with `ui_plugin`: a plugin key, `"core"` for the original interface, or `null` to follow the organization default. Only enabled plugins that declare `[ui]` are accepted.
- **Effective plugin:** the saved choice if available, else the organization default, else none. `GET /api/v1/preferences` returns `ui_plugin`, `ui_plugin_default` and `effective_ui_plugin`.
- **Listing:** `GET /api/v1/ui-plugins` (any signed-in user) returns key, label, description, whether it ships screens or colors, and a few preview colors.

Choosing a plugin installs and enables nothing.

## Loading

Colors: `index.html` links `/api/v1/public/ui/default.css` and, before React starts, switches it to `/api/v1/public/ui/<key>.css` when the browser remembers a choice (`localStorage` key `aphel.ui`). `core.css` and an absent default are empty.

Code: `GET /api/v1/public/wasm-runtime?ui=<key>` returns the bundles of ordinary plugins plus only the chosen UI plugin (or the organization default), so installing hundreds of UI plugins loads none of them for other users. After sign-in the web app compares the effective plugin with the loaded one and reloads once if they differ.

## Lifecycle

- Disabling a UI plugin removes it from the choices; saved references are kept and fall back.
- Uninstalling clears it from every user's preferences and from the organization default.
- Upgrading re-reads the manifest; there is no stored copy of the tokens.

## Trust

A UI plugin with a web bundle runs in the application document with the session of every user who selects it, and can redraw the frame and record pages. There is no isolation: install only code you trust. A UI plugin with only tokens runs no code.

## Packaging

A colors-only plugin is just a `plugin.toml` with `[ui]` and token tables, as shown above. A plugin with screens adds `web/index.js` (see [Web in React](../plugins/README.md#web-in-react)) whose default export calls `app.provideComponent(APP_FRAME_CONTRACT, Frame)` and/or `app.provideComponent(RECORD_PRESENTATION_CONTRACT, Presentation)`.

Pack with `aphel-plugin pack <dir> -o <name>.aphelplugin` and install in Settings › Modules. Each user then chooses it in Account › Appearance.
