# Design tokens

Generated from `back/modules/themes/tokens.json` by `make modules`. Do not edit by hand.

| Token | Group | Kind | Default | Dark |
| --- | --- | --- | --- | --- |
| `--radius` | shape | length | `0.625rem` | same |
| `--font-family-sans` | type | font | `'Inter', ui-sans-serif, system-ui, 'Segoe UI', Roboto, sans-serif` | same |
| `--font-family-heading` | type | font | `var(--font-family-sans)` | same |
| `--font-family-mono` | type | font | `ui-monospace, SFMono-Regular, Menlo, Consolas, monospace` | same |
| `--elevation-xs` | depth | shadow | `0 1px 2px 0 rgb(0 0 0 / 0.05)` | same |
| `--elevation-sm` | depth | shadow | `0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1)` | same |
| `--elevation-md` | depth | shadow | `0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)` | same |
| `--elevation-lg` | depth | shadow | `0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)` | same |
| `--overlay` | depth | color | `rgb(0 0 0 / 0.5)` | same |
| `--duration-fast` | motion | duration | `150ms` | same |
| `--control-height-sm` | size | length | `2rem` | same |
| `--control-height-md` | size | length | `2.25rem` | same |
| `--control-height-lg` | size | length | `2.5rem` | same |
| `--button-radius` | shape | length | `calc(var(--radius) - 2px)` | same |
| `--input-radius` | shape | length | `calc(var(--radius) - 2px)` | same |
| `--checkbox-radius` | shape | length | `4px` | same |
| `--badge-radius` | shape | length | `calc(var(--radius) - 2px)` | same |
| `--card-radius` | shape | length | `calc(var(--radius) + 4px)` | same |
| `--card-shadow` | depth | shadow | `var(--elevation-xs)` | same |
| `--card-border-width` | shape | length | `1px` | same |
| `--dialog-radius` | shape | length | `var(--radius)` | same |
| `--popover-radius` | shape | length | `calc(var(--radius) - 2px)` | same |
| `--popover-shadow` | depth | shadow | `var(--elevation-md)` | same |
| `--background` | color | color | `oklch(0.99 0.002 264)` | `oklch(0.17 0.012 264)` |
| `--foreground` | color | color | `oklch(0.21 0.02 264)` | `oklch(0.95 0.005 264)` |
| `--card` | color | color | `oklch(1 0 0)` | `oklch(0.21 0.014 264)` |
| `--card-foreground` | color | color | `oklch(0.21 0.02 264)` | `oklch(0.95 0.005 264)` |
| `--popover` | color | color | `oklch(1 0 0)` | `oklch(0.21 0.014 264)` |
| `--popover-foreground` | color | color | `oklch(0.21 0.02 264)` | `oklch(0.95 0.005 264)` |
| `--primary` | color | color | `var(--org-primary, oklch(0.55 0.2 264))` | `var(--org-primary-dark, oklch(0.72 0.14 264))` |
| `--primary-solid` | color | color | `var(--org-primary-solid, oklch(0.55 0.2 264))` | same |
| `--primary-foreground` | color | color | `var(--org-primary-foreground, oklch(0.985 0 0))` | same |
| `--secondary` | color | color | `oklch(0.96 0.006 264)` | `oklch(0.27 0.014 264)` |
| `--secondary-foreground` | color | color | `oklch(0.25 0.02 264)` | `oklch(0.95 0.005 264)` |
| `--muted` | color | color | `oklch(0.96 0.006 264)` | `oklch(0.25 0.012 264)` |
| `--muted-foreground` | color | color | `oklch(0.52 0.02 264)` | `oklch(0.7 0.015 264)` |
| `--accent` | color | color | `oklch(0.95 0.012 264)` | `oklch(0.28 0.016 264)` |
| `--accent-foreground` | color | color | `oklch(0.25 0.02 264)` | `oklch(0.95 0.005 264)` |
| `--destructive` | color | color | `oklch(0.56 0.22 27)` | `oklch(0.7 0.19 25)` |
| `--destructive-solid` | color | color | `oklch(0.56 0.22 27)` | same |
| `--destructive-foreground` | color | color | `oklch(0.985 0 0)` | same |
| `--success` | color | color | `oklch(0.5 0.13 155)` | `oklch(0.72 0.14 155)` |
| `--success-solid` | color | color | `oklch(0.5 0.13 155)` | same |
| `--success-foreground` | color | color | `oklch(0.985 0 0)` | same |
| `--warning` | color | color | `oklch(0.53 0.13 60)` | `oklch(0.8 0.14 75)` |
| `--warning-solid` | color | color | `oklch(0.8 0.15 80)` | same |
| `--warning-foreground` | color | color | `oklch(0.25 0.05 75)` | same |
| `--border` | color | color | `oklch(0.91 0.008 264)` | `oklch(0.3 0.014 264)` |
| `--input` | color | color | `oklch(0.66 0.015 264)` | `oklch(0.52 0.016 264)` |
| `--ring` | color | color | `var(--primary)` | same |
| `--sidebar` | color | color | `oklch(0.975 0.004 264)` | `oklch(0.19 0.013 264)` |
| `--sidebar-foreground` | color | color | `oklch(0.3 0.02 264)` | `oklch(0.85 0.01 264)` |
| `--sidebar-accent` | color | color | `oklch(0.93 0.012 264)` | `oklch(0.27 0.016 264)` |
| `--sidebar-accent-foreground` | color | color | `oklch(0.21 0.02 264)` | `oklch(0.97 0.005 264)` |
| `--sidebar-border` | color | color | `oklch(0.91 0.008 264)` | `oklch(0.28 0.014 264)` |
