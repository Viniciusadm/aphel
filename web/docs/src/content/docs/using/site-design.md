---
title: Customize the site
description: Change the look of the public site and blog without code — themes, styles, fonts, templates, headers, footers, patterns and menus.
sidebar:
  order: 12
---

The public site is drawn from **blocks**. Pages, posts and the parts around them (header, footer, sidebars) are block templates you can change in the browser, and one **styles document** sets colors, fonts and spacing for everything.

:::note
These screens need the **Site** and **Publication** modules and the `site.appearance.edit` / `site.design.edit` permissions (administrators have both). Nothing changes for visitors until you publish.
:::

## Pick a theme and a style

**Site → Appearance** shows ready-made styles as cards (Default, Editorial, Magazine, Minimal, Modern, Night, plus the variations of the active theme). A card changes colors, fonts, sizes and shapes; then fine-tune:

- **Colors** — the palette with a light and a dark value for each color (pick it or type the hex code), your own extra colors, and the color mode (follow the device, always light or always dark). A warning appears when text and background are too close to read. Notice boxes take their tone from the link and accent colors.
- **Typography** — fonts for headings and text, text size, line height, heading size and weight, link underline and button shape.
- **Fonts** — upload `.woff2` (or `.woff`) files, one per weight and style, and pick them as heading or text font. They are served by your own site.
- **Additional CSS** — for people who know CSS; applies to every page.
- **Shape and width** — card corners and shadow, text width and site width.

*Apply appearance* publishes; on **Site → Header and footer** the same two buttons also save the header and footer, so there is nothing else to save first. The site name, logo and icon are under **Identity**; without a name the site uses the organization name. *Back to the theme colors and fonts* drops your overrides. Installing a **theme plugin** (Editorial, Revista, Minimal, Estúdio or one exported from another site) and choosing it in *Theme* changes styles, templates, header and footer at once; your own changes stay on top.

## Edit pages in the site editor

**Site → Site editor** lists:

- **Templates** of every site page, grouped by plugin — for the blog: post list, post, category, tag, series and author, plus post layouts such as *with sidebar*, *cover behind the title*, *full width* and *minimal*.
- **Parts** — header, footer and any part you create.
- **Patterns** — reusable groups of blocks, such as the post card.
- **Menus** — extra menus for the `Menu` block.

The center shows the real page as visitors will see it, at desktop, tablet or phone width; type another address in the bar to preview a different post or page. Click any block in the preview (or in **Structure**) to edit it on the right:

- every block has **Style**: colors, spacing, borders, corners, shadow, typography, width (normal, wide, full screen), hiding on desktop, tablet or phone, and extra CSS classes;
- layout blocks (**Group**, **Row**, **Stack**, **Columns**, **Cover**, **Collapsible section**) hold other blocks;
- template blocks show the current item: **Title**, **Field** (summary, author, date, reading time…), **Item image**, **Categories and tags**, **Content**, **“Keep reading” link**, and **Item list** with its own card, pagination and layouts (grid, list, lead and grid, masonry, carousel);
- plugins add their own blocks — the blog brings author box, series navigation, previous/next, share buttons, table of contents, reading progress, categories and featured badge.

Keyboard: `Delete` removes, `Alt+↑/↓` moves, `Ctrl+D` duplicates, `Esc` selects the outer block, `Ctrl+Z` / `Ctrl+Shift+Z` undo and redo. Changes are saved as a draft automatically; **Publish** puts them on the site. **Back to default** returns to the plugin or theme version.

To give one category its own page, use **Create → New template**, choose *Category* and name it with the category address (for example `recipes`). To choose a layout for one post, open the post settings and pick **Page layout**.

**Save as pattern** turns the selected block into a pattern; **Insert pattern** adds a copy of any pattern. The **Pattern** block instead shows the pattern itself, so editing the pattern changes every page that uses it.

**Export as theme** downloads your published design as a theme plugin you can install on another Aphel site.
