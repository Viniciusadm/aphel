# Themes

A theme changes the look of the whole installation: colors, shapes, sizes, shadows, typography and motion. It is a JSON file of design tokens (plus optional fonts), installed by an administrator in **Settings › Organization › Themes**. There is no custom CSS and no code in a theme.

Every component of the UI library ([`@aphel/ui`](../ui/README.md)) reads those tokens, so the theme applies to the Aphel app, to its public pages (quotes, booking) and to every plugin that uses the library, without any change in the plugin. A plugin that draws its own interface is not affected, which is why using the library is recommended but optional.

The module is `themes` (`back/modules/themes`, enabled by default). The organization color (Settings › Organization) still wins over the theme's primary colors.

## How it is applied

The page links `/api/v1/public/themes/active.css` in its `<head>`. The response holds the tokens of the active theme and its `@font-face` rules, nothing else, and is empty when no theme is active. It overrides `html:root`, so the order of the stylesheets does not matter, and it carries an `ETag` that changes whenever the active theme does.

A plugin with a public web app (`/p/<key>/`) has its own `index.html` and must link the same stylesheet next to `/shared/ui.css`:

```html
<link rel="stylesheet" href="/shared/ui.css" />
<link rel="stylesheet" href="/api/v1/public/themes/active.css" />
```

Fonts are served at `/api/v1/public/themes/fonts/<sha256>.woff2` (same origin, cached forever).

## Package format (`aphel-theme`, version 1)

```json
{
  "format": "aphel-theme",
  "format_version": 1,
  "id": "acme.warm",
  "name": "Warm",
  "version": "1.0.0",
  "author": "ACME",
  "description": "Warm colors and a serif font.",
  "license": "MIT",
  "min_aphel_version": "0.1.0",
  "tokens": {
    "shared": {
      "radius": "0.25rem",
      "card-shadow": "none",
      "font-family-sans": { "families": ["Fraunces"], "fallback": "serif" }
    },
    "light": { "primary": "oklch(0.55 0.15 40)", "primary-solid": "oklch(0.55 0.15 40)" },
    "dark": {}
  },
  "fonts": [{ "family": "Fraunces", "weight": 400, "style": "normal", "format": "woff2", "data": "<base64>" }]
}
```

| Field | Rule |
| --- | --- |
| `id` | Lowercase letters, digits, `.`, `_`, `-`; up to 64 characters. `validate`, `activation`, `default` and `new` are reserved |
| `version` | Semantic version |
| `min_aphel_version` | Optional `X.Y.Z`; a newer requirement than this Aphel is refused |
| `tokens.shared` | Applies to light and dark. Shapes, sizes, depth, typography and motion go here |
| `tokens.light`, `tokens.dark` | Colors per scheme. The app is light only for now; dark values are stored for when dark mode is released |
| `fonts` | Up to 6 woff2 files of at most 400 KB, 2 MiB together, base64 encoded |

Token names are the ones in [`tokens.md`](../ui/tokens.md), without the `--` prefix. A token the host does not know is ignored with a warning, so a theme made for a newer Aphel still installs. A token you leave out keeps the default.

## Validation

The server validates every value by the token's kind and rejects the package with `theme_invalid` (and `details.fields`) otherwise. Nothing is stored as free text: the stylesheet is built from the validated data.

| Kind | Accepted |
| --- | --- |
| `color` | `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`, `hsl()`, `hsla()`, `oklch()`, `oklab()`, `transparent`, `currentColor`, or `var(--other-color-token)` |
| `length` | A number with `px`, `rem`, `em` or `%` (or `0`), `calc(var(--radius) ± N)`, or `var(--other-length-token)` |
| `shadow` | `none`, a list of `[inset] x y [blur [spread]] color` layers, or `var(--other-shadow-token)` |
| `duration` | A number with `ms` or `s` |
| `font` | `{ "families": [...], "fallback": "sans-serif" }` where each family is bundled in `fonts` or a known system font, or `var(--other-font-token)` |

Any value with `url(`, `;`, `{`, `}`, `\`, quotes, `@` or `/*`, or longer than 300 characters, is refused.

Text contrast is not enforced. The install dialog and the editor warn when a text token and its background are below 4.5:1.

## Installing and editing

- **Import:** Settings › Organization › Themes › Import theme. A validation report (problems, ignored tokens, fonts, contrast, replacement of an installed version) comes before the install. Installing does not activate.
- **Editor:** create a theme or duplicate one. The preview shows the UI library with the values being edited. Saving an active theme applies it immediately.
- **Export:** downloads the package, fonts included.
- **Catalog:** entries with `kind: "theme"` in the community catalog ([`catalog/README.md`](../../catalog/README.md)) install through the same report.

At most 20 themes can be installed. The active one cannot be removed (`theme_active`).

## API

Administration needs `themes.manage` (granted to the admin role). Every change publishes `theme.installed`, `theme.updated`, `theme.removed` or `theme.activated` and is audited.

| Route | Purpose |
| --- | --- |
| `GET /api/v1/themes` | Installed themes and the active one |
| `POST /api/v1/themes/validate` | Report without installing |
| `POST /api/v1/themes` | Install or replace |
| `PUT /api/v1/themes/{key}` | Save from the editor |
| `DELETE /api/v1/themes/{key}` | Remove |
| `POST /api/v1/themes/activation` | Activate a theme, or `{ "key": null }` for the default look |
| `GET /api/v1/themes/{key}/package` | Export |
| `POST /api/v1/catalog/themes/{id}/download` | Download and verify a catalog theme (needs `settings.manage`) |
| `GET /api/v1/public/themes/active.css` | Stylesheet of the active theme (public) |
| `GET /api/v1/public/themes/fonts/{sha256}.woff2` | A theme font (public) |

## Stability

The token contract is `back/modules/themes/tokens.json`; `make modules` generates `web/src/sdk/tokens.css`, `web/src/sdk/tokens.generated.ts` and [`docs/ui/tokens.md`](../ui/tokens.md) from it, and `make lint` checks they are in sync. Tokens and the package format only grow within version 1.
