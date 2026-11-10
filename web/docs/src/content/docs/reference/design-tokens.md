---
title: Design tokens
description: Every design token of the Aphel interface with its kind, group, default values and organization override property, and the rules for the values a UI plugin may set.
sidebar:
  order: 9
---

The Aphel interface is styled with a fixed vocabulary of **design tokens**: CSS custom properties for colors, radii, typography, shadows, motion and control sizes. Every base component reads its look from these tokens and never from fixed values, so changing a token restyles the whole interface. The vocabulary is fixed: a UI plugin can set values for these tokens but cannot define new ones.

The tokens are defined in `back/modules/wasm_runtime/tokens.json` (55 tokens) and mirrored in the stylesheet that the interface loads. The defaults below are the values of the built-in look.

## Where tokens come from

| Layer | What it does |
| --- | --- |
| Default stylesheet | Sets every token on `:root`, and the dark values of the color tokens under `[data-theme="dark"]`. |
| Organization color | The primary color chosen in **Settings, Organization** is turned into a small palette and written to the `--org-*` properties on the root element. The tokens that list an override property below read it with a fallback: `--primary: var(--org-primary, <default>)`. Removing the color removes the properties and the default applies again. |
| UI plugin | A runtime plugin that declares `[ui]` in its manifest can set values for tokens. Users choose an interface in their preferences and the administrator picks the organization default. The plugin's stylesheet is served from `/api/v1/public/ui/<key>.css`. |

If a UI plugin sets a token that has an organization override property, the value is emitted as `var(--org-..., <plugin value>)`, so the organization color still wins when one is set.

## Tokens

Each token is used as `var(--name)` in CSS. "Kind" is the type of value the token accepts. "Same" in the dark column means the token has no separate dark value.

### Color

Surface, text, state and brand colors. Colors have a light value and, when it differs, a dark value used under `[data-theme="dark"]`.

| Token | Kind | Default (light) | Dark | Organization override |
| --- | --- | --- | --- | --- |
| `--background` | color | `oklch(0.99 0.002 264)` | `oklch(0.17 0.012 264)` | - |
| `--foreground` | color | `oklch(0.21 0.02 264)` | `oklch(0.95 0.005 264)` | - |
| `--card` | color | `oklch(1 0 0)` | `oklch(0.21 0.014 264)` | - |
| `--card-foreground` | color | `oklch(0.21 0.02 264)` | `oklch(0.95 0.005 264)` | - |
| `--popover` | color | `oklch(1 0 0)` | `oklch(0.21 0.014 264)` | - |
| `--popover-foreground` | color | `oklch(0.21 0.02 264)` | `oklch(0.95 0.005 264)` | - |
| `--primary` | color | `oklch(0.55 0.2 264)` | `oklch(0.72 0.14 264)` | `--org-primary` (light), `--org-primary-dark` (dark) |
| `--primary-solid` | color | `oklch(0.55 0.2 264)` | same | `--org-primary-solid` (light) |
| `--primary-foreground` | color | `oklch(0.985 0 0)` | same | `--org-primary-foreground` (light) |
| `--secondary` | color | `oklch(0.96 0.006 264)` | `oklch(0.27 0.014 264)` | - |
| `--secondary-foreground` | color | `oklch(0.25 0.02 264)` | `oklch(0.95 0.005 264)` | - |
| `--muted` | color | `oklch(0.96 0.006 264)` | `oklch(0.25 0.012 264)` | - |
| `--muted-foreground` | color | `oklch(0.52 0.02 264)` | `oklch(0.7 0.015 264)` | - |
| `--accent` | color | `oklch(0.95 0.012 264)` | `oklch(0.28 0.016 264)` | - |
| `--accent-foreground` | color | `oklch(0.25 0.02 264)` | `oklch(0.95 0.005 264)` | - |
| `--destructive` | color | `oklch(0.56 0.22 27)` | `oklch(0.7 0.19 25)` | - |
| `--destructive-solid` | color | `oklch(0.56 0.22 27)` | same | - |
| `--destructive-foreground` | color | `oklch(0.985 0 0)` | same | - |
| `--success` | color | `oklch(0.5 0.13 155)` | `oklch(0.72 0.14 155)` | - |
| `--success-solid` | color | `oklch(0.5 0.13 155)` | same | - |
| `--success-foreground` | color | `oklch(0.985 0 0)` | same | - |
| `--warning` | color | `oklch(0.53 0.13 60)` | `oklch(0.8 0.14 75)` | - |
| `--warning-solid` | color | `oklch(0.8 0.15 80)` | same | - |
| `--warning-foreground` | color | `oklch(0.25 0.05 75)` | same | - |
| `--border` | color | `oklch(0.91 0.008 264)` | `oklch(0.3 0.014 264)` | - |
| `--input` | color | `oklch(0.66 0.015 264)` | `oklch(0.52 0.016 264)` | - |
| `--ring` | color | `var(--primary)` | same | - |
| `--sidebar` | color | `oklch(0.975 0.004 264)` | `oklch(0.19 0.013 264)` | - |
| `--sidebar-foreground` | color | `oklch(0.3 0.02 264)` | `oklch(0.85 0.01 264)` | - |
| `--sidebar-accent` | color | `oklch(0.93 0.012 264)` | `oklch(0.27 0.016 264)` | - |
| `--sidebar-accent-foreground` | color | `oklch(0.21 0.02 264)` | `oklch(0.97 0.005 264)` | - |
| `--sidebar-border` | color | `oklch(0.91 0.008 264)` | `oklch(0.28 0.014 264)` | - |

### Shape

Corner radii and border widths. Component radii are derived from `--radius`.

| Token | Kind | Default (light) |
| --- | --- | --- |
| `--radius` | length | `0.625rem` |
| `--button-radius` | length | `calc(var(--radius) - 2px)` |
| `--input-radius` | length | `calc(var(--radius) - 2px)` |
| `--checkbox-radius` | length | `4px` |
| `--badge-radius` | length | `calc(var(--radius) - 2px)` |
| `--card-radius` | length | `calc(var(--radius) + 4px)` |
| `--card-border-width` | length | `1px` |
| `--dialog-radius` | length | `var(--radius)` |
| `--popover-radius` | length | `calc(var(--radius) - 2px)` |

### Typography

Font stacks for body text, headings and code.

| Token | Kind | Default (light) |
| --- | --- | --- |
| `--font-family-sans` | font | `'Inter', ui-sans-serif, system-ui, 'Segoe UI', Roboto, sans-serif` |
| `--font-family-heading` | font | `var(--font-family-sans)` |
| `--font-family-mono` | font | `ui-monospace, SFMono-Regular, Menlo, Consolas, monospace` |

### Depth

Shadows and the overlay color behind dialogs.

| Token | Kind | Default (light) |
| --- | --- | --- |
| `--elevation-xs` | shadow | `0 1px 2px 0 rgb(0 0 0 / 0.05)` |
| `--elevation-sm` | shadow | `0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1)` |
| `--elevation-md` | shadow | `0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)` |
| `--elevation-lg` | shadow | `0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)` |
| `--overlay` | color | `rgb(0 0 0 / 0.5)` |
| `--card-shadow` | shadow | `var(--elevation-xs)` |
| `--popover-shadow` | shadow | `var(--elevation-md)` |

### Motion

Animation timing.

| Token | Kind | Default (light) |
| --- | --- | --- |
| `--duration-fast` | duration | `150ms` |

### Size

Heights of form controls and buttons.

| Token | Kind | Default (light) |
| --- | --- | --- |
| `--control-height-sm` | length | `2rem` |
| `--control-height-md` | length | `2.25rem` |
| `--control-height-lg` | length | `2.5rem` |

## Rules for plugin values

Values set by a UI plugin are validated when the plugin is installed. A token is addressed by its name without the leading dashes.

| Kind | Accepted values |
| --- | --- |
| `color` | A hex color (`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`), `rgb()`, `rgba()`, `hsl()`, `hsla()`, `oklch()`, `oklab()`, `transparent`, `currentColor`, or a reference to another color token. |
| `length` | A number with `px`, `rem`, `em` or `%` (or `0`), `calc(var(--radius) +/- <length>)`, or a reference to another length token. |
| `duration` | A number with `ms` or `s`, such as `150ms` or `0.2s`, or a reference to another duration token. |
| `shadow` | `none`, a comma-separated list of layers in the form `[inset] x y [blur [spread]] color` with `px` or `rem` lengths, or a reference to another shadow token. |
| `font` | An object `{ "families": [...], "fallback": "..." }` with 1 to 6 family names and a generic fallback, or a reference to another font token. A family must be one bundled with the plugin or a known system font. |

A reference is written `var(--other-token)` and must point to a token of the same kind. Every value is limited to 300 characters and cannot contain `;`, braces, backslashes, quotes, angle brackets, `@`, comments, `url(` or `expression`. The generic fallbacks are `sans-serif`, `serif`, `monospace`, `system-ui`, `ui-sans-serif`, `ui-serif` and `ui-monospace`. The known system fonts are Inter, Arial, Helvetica, Georgia, Times New Roman, Courier New, Verdana, Tahoma, Trebuchet MS, Segoe UI, Roboto, Menlo, Consolas, Monaco, SF Pro Text, Helvetica Neue and Noto Sans.

How to declare tokens, bundle fonts and provide screens in a UI plugin is described in [UI plugins](/_aphel/docs/plugins/ui-plugins/), and the components that consume the tokens are in [UI library](/_aphel/docs/plugins/ui-library/).
