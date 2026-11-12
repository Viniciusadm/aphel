---
title: Site pages
description: Declare public pages (blog posts, catalogs, landing pages) that the site module renders on the server from your published records, with no backend code.
sidebar:
  order: 53
---

Site pages are public HTML pages rendered on the server by the `site` module. A plugin declares them in `plugin.toml` and ships the templates in its package. Aphel looks up the records, applies the publication filter, renders the template inside the site theme and answers with complete HTML, so search engines and link previews see the content. A plugin that only publishes pages needs no `backend.wasm`.

Use site pages for content that visitors read: posts, products, categories, portfolios. Use a [public web app](/_aphel/docs/plugins/public-web-apps/) for an interactive single-page tool.

The `site` module must be enabled. An administrator chooses where the site answers under Site → *Address and cache*: its path (`/s` by default, or `/` when the app moves to a path such as `/app`) and, optionally, its own domain. Page paths in this guide are relative to that address.

## Declare what is public

Nothing becomes public by accident. Each entity whose records appear on the site is listed in `[[site.public]]` with the fields templates may read and the conditions a record must meet.

```toml
[[site.public]]
entity = "post"
fields = ["title", "slug", "excerpt", "body", "status", "published_at"]
where = { status = "published" }
```

| Key | Type | Notes |
| --- | --- | --- |
| `entity` | string | An entity of this plugin. |
| `fields` | array of strings | Fields of the entity that templates receive. `id`, `created_at` and `updated_at` are always available. Other fields never reach the template. |
| `where` | table | Equality conditions applied to every read on the site, including direct links and images. A field used here must also be in `fields`. |
| `filter` | table | A record filter (the same tree as the record API's `filter`) added to `where`, for conditions equality cannot express, such as a related record: `{ related = { relationship = "parent", filter = { field = "state", operator = "eq", value = "active" } } }`. See [relations](/_aphel/docs/plugins/entities-and-fields/#record-fields-and-references). An invalid filter makes nothing public. |
| `images` | boolean | When `true`, image attachments of a public record (JPEG, PNG, WebP, GIF) become public too and reach templates as `images`. Default `false`. |

A record that fails the conditions does not exist for the site: its page answers 404. The consent dialog lists the public entities and fields.

## Declare the pages

```toml
[[site.pages]]
key = "index"
path = "/blog"
template = "site/index.html"
entity = "post"
list = { sort = "-published_at", per_page = 10 }
title = "Blog"

[[site.pages]]
key = "post"
path = "/blog/{slug}"
template = "site/post.html"
entity = "post"
match = "slug"
title = "{{ record.title }}"
```

| Key | Type | Notes |
| --- | --- | --- |
| `key` | string | Unique in the plugin. Lowercase letters, digits and `_`. Used by `url_for`. |
| `path` | string | Starts with `/`. Segments use lowercase letters, digits, `-`, `_` and `.`, and cannot start with `_`. A parameter looks like `{slug}`. `/sitemap.xml`, `/robots.txt`, `/busca` and `/search` are reserved. |
| `template` | string | A file of the package under `site/`, ending in `.html`. |
| `entity` | string | Optional. An entity listed in `[[site.public]]`. |
| `match` | string | One record: the public field compared with the only path parameter, which must have the same name (`/blog/{slug}` with `match = "slug"`). |
| `where` | table | One record chosen by equality conditions on public fields, for a page without parameters (`path = "/"` with `where = { is_home = true }`). When several records match, the most recently updated wins; when none does, the page answers 404. |
| `list` | table | A paginated list: `sort` (public fields, `-` for descending, comma separated) and `per_page` (1 to 50, default 10). The page reads `?page=N`. |
| `feed` | boolean | Optional, list pages only. Serves an RSS 2.0 feed of the 20 newest records at the page address plus `/feed.xml` (`/blog/feed.xml`, `/blog/categoria/news/feed.xml`) and adds its `<link rel="alternate">` to the page head. See [Feeds](#feeds). |
| `title` | string | Optional. A template expression rendered with the page context, used in `<title>` and in the share previews. |
| `data` | string | Optional. Path of a GET route of the plugin declared with `site = true`, using the page parameters (`/products/{slug}`). Its JSON answer reaches the template as `data`; a 404 answer makes the page 404. See [Dynamic pages](#dynamic-pages). |
| `seo` | table | Optional. `description` and `image` are template expressions; `noindex = true` keeps the page out of search engines and the sitemap; `schema`, `author`, `published`, `modified` and `structured_data` shape the JSON-LD. See [Search engines](#search-engines). |

A page without `entity` is static and has no parameters, unless it declares `data`. A page with `entity` needs exactly one of `match`, `where` or `list`.

Two plugins cannot serve the same path shape: installing the second fails with `409 site_route_conflict`. The home page `/` follows the same rule; the organization usually points it at a target instead (see [Navigation and home page](#navigation-and-home-page)). Paths whose first segment belongs to a public app page (`p`, `book`, `quote`) are refused the same way. Paths are matched from the most specific to the least specific, so `/blog/archive` wins over `/blog/{slug}`.

## Write the templates

Templates use [MiniJinja](https://docs.rs/minijinja), a Jinja2 dialect. Output is HTML-escaped by default. Extend the site theme with `{% extends "layout.html" %}` and fill `{% block content %}`; the theme also has `title` and `head` blocks. Include your own partials by their package path: `{% include "site/_card.html" %}`. The install checks that every `extends`, `include`, `import` and `from` with a literal name points to a template of the package or of the built-in theme (`layout.html`, `aphel/head.html`); a missing one fails with `plugin_site_invalid`. Use `ignore missing` for optional partials.

```html
{% extends "layout.html" %}
{% block content %}
<h1>Blog</h1>
{% for post in records %}
  <h2><a href="{{ url_for('post', slug=post.slug) }}">{{ post.title }}</a></h2>
  <p>{{ post.published_at | date }}</p>
{% endfor %}
{% if pagination.next_url %}<a href="{{ pagination.next_url }}">{{ site.texts.next }}</a>{% endif %}
{% endblock %}
```

| Variable | Content |
| --- | --- |
| `site` | `name`, `locale`, `color`, `prefix` (the site path, empty at the root), `home`, `year` and `texts` (`previous`, `next`, `back_home` and the error page texts in the organization language). |
| `page` | `owner`, `key`, `path`, `params` and the rendered `title`. |
| `record` | The record of a `match` page, with the public fields only, plus `images` when the entity exposes them: a list of `{ id, name, type, url }`, at most 100. |
| `records`, `pagination` | The records of a `list` page and `page`, `pages`, `total`, `previous_url`, `next_url`. |

| Function or filter | Use |
| --- | --- |
| `url_for(key, **params)` | Address of a page of the same plugin, including the site path. |
| `site_path(path)` | Address of a site path typed by people (`/contact`, `/book/intro`): adds the site path and the language code, except for public app pages (`book`, `quote`, `p`), which keep the site path only. Other values (`https://…`, `#anchor`) are returned unchanged. |
| `embed(kind, key)` | Places something another module publishes, rendered on the server: `embed('form', <form id>)` (a form of the Forms module, which also works without JavaScript) or `embed('booking', <slug>)` (a card with a button to the booking page). Unknown or inactive items render nothing. The kinds and items available come from `GET /api/v1/site/embeds`. |
| `site_list(owner, entity, sort=, limit=, where=, exclude=, heading=)` | The published records of a public entity of any plugin, each rendered with that plugin's card template `site/cards/<entity>.html`, inside `<ul class="site-list">`. `sort` uses public fields (default `-created_at`) or `newest`/`oldest`, which order by `published_at` when the entity exposes it and by `created_at` otherwise; `limit` is 1 to 24 (default 6), `where` is a map of public fields to values (`where={'collection': prefix.id}`); a list value matches any of its items, also inside list fields such as `references` (`where={'authors': [record.id]}`). Entities without a card template render nothing; `GET /api/v1/site/collections` lists the ones that have one, with `scope_field` and `scopes` when a public `belongs to` relation lets the list be limited to one parent record (the *Collection* block uses them to show only the Blog, for example). `exclude` leaves one record id out (the current record, for related posts: `site_list('content', 'entry', where={'terms': record.terms}, exclude=record.id, limit=3)`), and `heading` adds an `<h2 class="site-list-heading">` above the list only when it has records. |
| `value \| markdown` | Renders Markdown and removes unsafe HTML (scripts, event handlers). |
| `value \| date(format)` | Formats an ISO date or date-time. Default `%d/%m/%Y`. |
| `text \| from_json` | Parses a JSON text (up to 512 KB and 16 levels deep) into lists and objects, for content stored as JSON in a `long_text` field: `{% for block in record.blocks \| from_json %}`. Empty text gives nothing; invalid JSON fails the render. |
| `blocks \| reading_time` | Minutes needed to read a JSON block document (200 words per minute, at least 1): `{{ record.body \| reading_time }} {{ site.texts.reading_time }}`. |
| `image \| image_url(width)` | Address of an image, resized to the smallest of 320, 640, 960, 1280 or 1920 pixels that is at least `width` wide. Without `width`, the original file. |

Templates are compiled when the plugin is installed: a syntax error, a missing template or a page that breaks the rules above fails the install with `422 plugin_site_invalid`, listing every problem. Templates cannot reach the network, files or the database: they only see the variables above. A render that runs too long stops and the visitor sees the theme's error page.

## How pages are served

- Only `GET` and `HEAD` are answered.
- Every response carries a content security policy that allows scripts, fonts and connections only from the same origin (plus the import map, by hash), images from the same origin, `data:` and `https:`, and the origins the page's plugin and the active theme declare in `[site.csp]`.
- Unknown paths, records that fail the publication conditions and list pages past the last one answer 404 with the theme's page.
- Rendered pages are kept in a cache in each server process. A change to a public record (create, update, delete, archive, restore) records the pages to clear in the same transaction as the change, so every process stops serving them within about a second; attachments and organization changes (name, color, settings, which clear every page) follow a few seconds later. When a process cannot read those records it stops serving from its cache instead of serving old pages. Plugin installs and upgrades start a new cache.
- Pages carry an `ETag` and `Cache-Control: public, max-age=60, stale-while-revalidate=600`, and answer `304` to a matching `If-None-Match`. Error pages are never cached.
- Renders (pages not in the cache) are limited to 120 per minute per client address; above that the site answers `429`.

## Pages under a record's address

A page can live under an address that comes from a record, such as `/blog/{slug}` where `blog` is the address of a collection chosen by people. Declare the record entity in `prefix`:

```toml
[[site.pages]]
key = "entry"
path = "/{slug}"
template = "site/entry.html"
entity = "entry"
match = "slug"
prefix = { entity = "collection", field = "path", where = { state = "active" }, scope = "collection" }
```

| Key | Notes |
| --- | --- |
| `entity` | A public entity of this plugin whose records give the first segment of the address. |
| `field` | The public field of that entity holding the segment. |
| `where` | Equality conditions the prefix record must meet, on public fields. |
| `scope` | The public field of the page's own entity that points to the prefix record; record and list pages only see records of that prefix. |

The page answers at `/<segment>` + `path` for every prefix record. Templates receive the prefix record as `prefix`, and `url_for` takes it as a parameter: `url_for('entry', prefix=prefix.path, slug=record.slug)`. Records listed by `site_list` or a list page of the same entity carry `site_prefix`, the segment of their prefix record, for links from cards: `url_for('entry', prefix=record.site_prefix, slug=record.slug)`. Prefix pages work for record (`match`) and list pages, not for customer pages.

### Category and tag pages

A list page can show the records of one public term of a `tagging` vocabulary, and of the terms below it when the vocabulary is hierarchical. Add `term` and put `{term}` in the path; it receives the term address:

```toml
[[site.pages]]
key = "category"
path = "/categoria/{term}"
template = "site/collection.html"
entity = "entry"
list = { sort = "-published_at", per_page = 12 }
prefix = { entity = "collection", field = "path", where = { state = "active" }, scope = "collection" }
term = { field = "terms", vocabulary = "categories" }
title = "{{ term.name }} · {{ prefix.name }}"
```

| Key | Notes |
| --- | --- |
| `field` | A public `references` field of the page's entity holding the term ids. |
| `vocabulary` | Key of a public vocabulary. Terms of other vocabularies, or of vocabularies that are not public, answer 404. |

The template receives the term as `term` (`id`, `vocabulary`, `name`, `slug`, `parent_id`). The sitemap lists one address per term of the vocabulary.

Every list page of an entity with term pages also receives `vocabularies`: for each vocabulary used by a `term` page of that entity, its public terms (`vocabularies.categories`), to draw a category menu with `url_for('category', prefix=prefix.path, term=category.slug)`.

Records of an entity that keeps a selection of terms (`tagging.classifiable` with `published`) carry `site_terms` when that field is public: the public terms of the selection, in order, with the same keys as `term`. Use it to show and link categories and tags: `url_for('category', prefix=record.site_prefix, term=term.slug)`. Without `tagging`, `site_terms` is empty and term pages answer 404. Renaming or moving a term refreshes the cached pages that use it.

A literal path of a manifest wins over a prefix, and a prefix wins over a page that is only a parameter (such as `/{slug}`). The site refuses to save a prefix record whose segment is a literal path of the site, a reserved address or the address of a record served by a one-parameter page, and refuses the opposite too, with `422 site_path_taken`.

## Cards

A plugin lets other plugins' pages list its records by shipping `site/cards/<entity>.html` for a public entity. The template receives `record` (public fields and `images`), `site` and a `page` whose `owner` is your plugin, so `url_for` points to your pages:

```html
<article><h3><a href="{{ url_for('post', slug=record.slug) }}">{{ record.title }}</a></h3><p>{{ record.excerpt }}</p></article>
```

`site_list` and `embed` are filled after the page is rendered, so they cannot be used inside `title` or `seo`. Pages that list records are refreshed when a record of that entity changes; pages with embeds stay in the cache for at most 60 seconds.

## Content blocks

Plugins that store structured content as blocks (hero, text, image, text with image, buttons, spacer, quote, code, callout, gallery, columns, FAQ, testimonials, contact, video, external form, form, booking, collection) share one implementation instead of shipping their own:

| Part | Where | What it gives |
| --- | --- | --- |
| Validation | Rust crate `aphel-blocks` in `sdk/rust/aphel-blocks` (feature `plugin` for `PluginError` conversions) | `blocks::normalize` (known types and fields, text limits, safe links, at most 300 blocks and 400 000 characters), `blocks::image_ids`/`replace_images`, `slug::check`/`slug::suggest`. Errors use the codes `block_*`, `blocks_*` and `slug_*`; `BlockError::at(field)` points them at your field |
| Editor | `@aphel/web-sdk` | `BlockForm`, `AddBlock`, `BlockImageField`, `BLOCK_ICONS`, the `Block` types, `newBlock`/`parseBlocks`/`serializeBlocks`, `blockProblems`/`blockSummary` and `useBlocksValidation`, which loads the same Rust validation compiled to WebAssembly. Images are attachments of the record you pass as `record: { recordType, recordId }`. Labels come from the `site-blocks` namespace |
| Rendering | Built-in templates of the `site` module | `aphel/blocks.html` renders a list of blocks and `aphel/blocks_style.html` their styles |

A page template only wraps the catalog with its own data:

```html
{% extends "layout.html" %}
{% block head %}{% include "aphel/blocks_style.html" %}{% endblock %}
{% block content %}{% with blocks = record.blocks | from_json, images = record.images %}{% include "aphel/blocks.html" %}{% endwith %}{% endblock %}
```

`images` is the list of public images of the record (`images = true` on the exposure). Every block is wrapped in an element whose id is the block id, so links such as `#contact` jump to it; unknown block types render nothing.

## Dynamic pages

Server-rendered pages cover what visitors read. For what they do (a cart, a like button, a shipping quote), a plugin with a backend adds **site routes** and **islands**.

### Site routes

```toml
[[routes]]
method = "POST"
path = "/cart/items"
site = true
```

A route with `site = true` answers at `/_aphel/site/api/<plugin key>/<path>` on every host of the installation, so scripts on the site reach it from the same origin. It runs like a public route: with the identity of the user who installed the plugin, limited by the plugin capabilities, and rate limited to 60 requests per minute per client address and 600 per minute per plugin. It cannot require a permission and is either public or a site route, not both. Requests other than `GET`, `HEAD` and `OPTIONS` must carry the header `X-Aphel-Site: 1`, which the island loader adds; without it they answer `403 site_request_forbidden`. Bodies are JSON, up to 1 MB.

### Data from the backend

A page can take its data from a GET site route instead of an entity:

```toml
[[site.pages]]
key = "product"
path = "/product/{slug}"
template = "site/product.html"
data = "/products/{slug}"
title = "{{ data.name }}"
```

The route's JSON is `data` in the template (and in `title` and `seo`). A `404` from the route makes the page 404; any other error shows the error page. Such pages stay in the cache for at most 60 seconds, because Aphel cannot know when the data behind them changes.

### Islands

An island is an ES module in the package at `site/islands/<name>.js` (lowercase letters, digits, `-` and `_`; at most 30). The plugin must declare `[site]`, even empty when it has no pages. A template places it with `island`:

```html
{{ island("like-button", slug=page.params.slug, label="Like") }}
```

This writes `<div data-island="<plugin>:<name>" data-props="…">`. The island loader (included by `aphel/head.html`) imports `/_aphel/site/islands/<plugin>/<name>.js` and calls its default export with the element, the props and a `site` object whose `api(path, init)` calls the plugin's site routes (JSON in and out, with the CSRF header):

```js
import { createElement, useState } from 'react'
import { createRoot } from 'react-dom/client'

function Like({ slug, label, site }) {
  const [count, setCount] = useState(null)
  const like = async () => setCount((await site.api(`/likes/${slug}`, { method: 'POST' })).likes)
  return createElement('button', { onClick: like }, `${label} (${count ?? '…'})`)
}

export default function mount(element, props, site) {
  createRoot(element).render(createElement(Like, { ...props, site }))
}
```

`react`, `react/jsx-runtime`, `react-dom`, `react-dom/client` and `@aphel/ui` resolve through the site's import map to the same files the application uses; anything else must be bundled into the island. A plain name refers to an island of the page's own plugin, and asking for an unknown one fails the render. A qualified name places an island of another plugin, `island('comments:thread', record_type='content.entry', record_id=record.id)`, which keeps calling its own plugin's site routes; when that plugin is not installed or has no such island, it renders nothing, so templates can offer optional features.

### Content security policy

```toml
[site.csp]
script = ["https://js.stripe.com"]
connect = ["https://api.stripe.com"]
frame = ["https://js.stripe.com"]
```

`script`, `style`, `img`, `font`, `connect` and `frame` accept up to 10 origins each, written as `https://host`, `https://host:port` or `https://*.domain`. They apply to the pages of the plugin that declares them, and to every page when the plugin is the active theme. The consent dialog lists them.

## Customer accounts

Visitors can have an account on the site, separate from the people who use Aphel. Accounts turn on by themselves when an installed plugin has customer pages, and an administrator can also turn them on in Site → *Customers*.

The site then serves, under `/conta` (`/account` when the organization language is not Portuguese): sign in with password or with a one-time link by email, sign up, password reset, email confirmation, and *My account* (name, sign out, close account). Pages are rendered by the site theme; emails go through the organization's outgoing email. The session is an `HttpOnly`, `SameSite=Lax` cookie valid for 30 days of use; form posts must come from the site itself, and sign-in attempts are rate limited.

```toml
[[site.pages]]
key = "orders"
path = "/orders"
template = "site/orders.html"
entity = "order"
list = { sort = "-created_at" }
customer_field = "customer"

[[site.account]]
key = "orders"
label = "My orders"
page = "orders"
```

| Key | Effect |
| --- | --- |
| `requires_customer = true` | The page needs a signed-in customer; visitors go to sign in and come back. |
| `customer_field` | Also restricts the records to those whose field holds the customer id (a public field of the entity). Implies `requires_customer`. |
| `[[site.account]]` | Links shown in *My account* (`key`, `label`, `page`); the page must need a customer and have no parameters. |

Templates receive `customer` (`{ id, email, name, email_verified }` or nothing) and the theme gets `site.account` (`home`, `login`, `signup`, `logout`, `password`, `close` addresses) to show *Sign in* or the customer name. Pages seen by a signed-in customer are never cached and answer `Cache-Control: private, no-store`; anonymous pages add `Vary: Cookie`.

Site routes receive the customer in `request.customer` (see [Routes](/_aphel/docs/plugins/routes/#the-request)), so a route can create an order for whoever is signed in. Store the customer id in a field of your entity (for example `customer`), or point a `record` field at the entity `site.customer`.

Customers are the record type `site.customer` (name, email, linked person, confirmation and last sign-in dates), readable with the permission `site.customers.read` and listed in Site → *Customers*. Account creation, changes and closing publish `site.customer.created|updated|deleted` with an audit entry. Closing an account, or a privacy request for it, erases the name, email and password and signs the customer out everywhere.

When the `people` module is enabled, Site → *Customers* can link customers to people: once a customer confirms the email (or signs in with a confirmed one), the account points (`person_id`) to the person with the same email, or to a new contact owned by the user the organization chose. Without an owner, only existing people are linked. Merging people moves the link to the person that stays; purging the person from the trash clears it.

## Navigation and home page

The site owns its structure; plugins do not declare menu items. In Site → *Navigation* the organization builds the menu and chooses the home page:

- A menu item points to a **target** (a page of a plugin without parameters, or a public record that has an address, such as a page, a collection or an entry) or to an address (a site path or an `https://` address). Items can be grouped one level deep. A label left empty uses the title of the target in the language of the request; labels per language come from Site → *Header and footer*.
- Targets are stable references: `record:<record type>:<id>` or `page:<owner>:<page key>`. When the address of the record changes, the menu follows it; when the record stops being public, its item disappears. `GET /api/v1/site/targets` lists them with their current path.
- The home page (`home` in the `site` setting) is a target too. `/` shows the page of that target, with `/` as its canonical address and pagination; without a choice, a plugin page declared at `/` answers, and without one the theme shows a welcome screen with the site name, tagline and menu.

Site → *Overview* (`GET /api/v1/site/status`, `settings.manage`) sums it up for the organization: the public address, whether the home page resolves, the number of menu items, whether the site has a name and a logo, published and draft counts for every public entity with `publication_state`, and the five latest published records with their address. Packs can set the home page with the `site.home` provision kind (see [Packs](/_aphel/docs/plugins/packs/)).

Editors can read the structure with `GET /api/v1/site/structure` and change it with `PUT /api/v1/site/home` and `PUT /api/v1/site/menu` (`settings.manage`). Links to public app pages (`/book/...`, `/quote/...`, `/p/<key>/`) keep the site address without the language code. Templates read the menu as `site.menu`, a list of `{ label, url, active, children }` where `children` has the same shape and `url` is null for groups; `active` is true when the visitor is on that page, below it, or on one of the children.

The theme also receives `site.tagline` and `site.logo_url` (the organization logo, served at `/_aphel/site/logo`).

### Header and footer

The *Site → Header and footer* screen fills two more objects, so every theme can draw them its own way:

| Variable | Contents |
| --- | --- |
| `site.header` | `{ align, cta }`: `align` is `left` or `center`; `cta` is `{ label, url }` for a highlighted button, or nothing. |
| `site.footer.text` | List of lines of the footer text (plain text; join them with `<br>`). |
| `site.footer.contact` | `{ phone, whatsapp, email, address }` or nothing. `phone`, `whatsapp` and `email` are `{ label, url }` (`tel:`, `https://wa.me/…`, `mailto:`); `address` is `{ lines, url }` with a map link. |
| `site.footer.social` | List of `{ network, label, url }`; `network` is `instagram`, `facebook`, `whatsapp`, `youtube`, `tiktok`, `linkedin` or `x`. |
| `site.footer.links` | List of `{ label, url, active }`. |
| `site.footer.credit` | `true` when the organization wants the “© year name” line. |
| `site.footer.empty` | `true` when there is no text, contact, network or link. |

`{% from "aphel/icons.html" import social_icon %}` gives `social_icon(network)`, an inline SVG icon in `currentColor` for each network. In an extra language, the tagline, the footer text, the button text and the labels of the organization's menu and footer links come from the translations saved on that screen; an empty translation falls back to the default text.

## Themes

A plugin becomes a theme with `[site.theme]` and a layout in its package:

```toml
[site.theme]
name = "Editorial"
description = "Serif type and a centered menu."
```

| File | Role |
| --- | --- |
| `site/theme/layout.html` | Required. Replaces `layout.html`: every page that extends it gets this frame. Keep a `{% block content %}`; `title` and `head` blocks are recommended. |
| `site/theme/not_found.html`, `site/theme/error.html` | Optional. Replace the 404 and error pages. They receive `site` and an empty `page`. |
| `site/theme/assets/*` | CSS, images and fonts, served at `/_aphel/site/themes/<plugin key>/<file>`. Link them with `{{ theme_asset('style.css') }}`. |

Include `{% include "aphel/head.html" %}` in the `<head>` of the layout: it writes the canonical, description, Open Graph and Twitter tags, the import map and the island loader, and, when the organization chooses a thin scrollbar in the site color (Site → *Appearance*, `site.scrollbar = "themed"`), the scrollbar styles. Partials of the theme are included by their package path (`{% include "site/theme/_header.html" %}`). The organization picks the theme in Site → *Appearance*; only one is active, and the built-in theme is used when none is chosen or the chosen plugin is disabled. A theme plugin needs no entities or pages.

Site → *Appearance* → *Preview* opens the site with another theme only in that browser, for 30 minutes, without changing it for visitors. Preview pages are never cached and show a banner with a link that ends the preview.

## Drafts

A plugin that keeps a draft next to the published content (for example `draft_blocks` next to `blocks`) can let people see the draft before publishing. Its web bundle calls `POST /api/v1/site/preview` with `record` and opens the returned `url` (in a new tab, or in an `iframe` when the site and the app share an origin):

```json
{
  "record": { "record_type": "blog.post", "record_id": "0192…", "fields": { "body": "draft_body" } },
  "path": "/blog/my-post"
}
```

In that browser, for 30 minutes, the pages of that record show it even when it fails the `where` conditions of `site.public`, with each public field named in `fields` replaced by the value of the other field (and `<field>_<code>` by `<source>_<code>` in other languages); its images are served too. Only that record changes: lists and other records stay as published. The caller needs permission to update the record (or `settings.manage`), the targets must be public fields and the sources fields the caller can read. The answer also carries `page_url`, the public address of `path`.

### Live preview

An editor can show unsaved changes without reloading the preview. `POST /api/v1/site/preview/render` renders `path` with literal values for the record and answers `{ "html": "<!doctype html>…" }`, writing nothing:

```json
{
  "record": { "record_type": "blog.post", "record_id": "0192…", "values": { "body": "<p>Unsaved text</p>", "title": "Hello" } },
  "path": "/en/blog/my-post"
}
```

The keys of `values` are public fields; they replace what the page shows in the language of `path`. At most 10 values, each a string or `null` up to 64 KB. Permissions match `POST /site/preview`, and each user may render 300 times per minute.

Draft previews load `/_aphel/site/preview.js`. Once loaded it posts `{ "type": "aphel:preview-ready" }` to the parent window. The parent (same origin only) answers with `{ "type": "aphel:preview-html", "html", "anchor" }`. The script swaps the body and the head styles in place, keeps the scroll position (or scrolls to `#anchor`), runs the new scripts again and mounts new islands. When the ready message does not arrive, for example because the site has its own host, the editor should fall back to reloading the frame after saving.

## Languages

The site speaks the organization language. In Site → *Languages* an administrator adds up to five more (Portuguese, English, Spanish, French, German, Italian). Each extra language answers under its code after the site address: `/s/en/blog`, `/s/es/blog`. The default language keeps the addresses without a code.

For each request in an extra language:

- `site.language` (and `site.locale`, used in `<html lang>`) is the language code; `site.prefix`, `site.home`, `url_for`, pagination, the menu and the account pages carry the language code, so links stay in the language.
- A public field declared with `translatable = true` takes the text stored for the language code in its `<field>_translations` map (`{"en": "Hello", "es": "Hola"}`) when it is filled. Otherwise the original value is used. The map is public together with its field and needs no entry in `fields`.
- Menu and *My account* labels come from `labels`.
- Theme texts are in the language of the page (Portuguese, English, Spanish, French, German or Italian); account pages are in Portuguese for `pt*` languages and in English for the others.

Editors learn the active languages, with the code used as the key of translation maps and the address prefix, from `GET /api/v1/site/languages` (any signed-in user). A draft preview (see [Drafts](#drafts)) maps `<field>_translations` from `<source>_translations` like any other field, so `path = "/en/about"` with `{"title_translations": "draft_title_translations"}` previews the English draft. `aphel/head.html` writes `<link rel="alternate" hreflang>` for every language plus `x-default`, and `site.alternates` lists `{ code, url, path, current }` for a language switcher (the built-in theme shows one in the header). The sitemap lists every page in every language. Values used in `match` (a slug) are shared by all languages.

## Search engines

```toml
[[site.pages]]
key = "post"
path = "/blog/{slug}"
template = "site/post.html"
entity = "post"
match = "slug"
title = "{{ record.title }}"
seo = { description = "{{ record.excerpt }}", image = "{% if record.images %}{{ record.images[0] | image_url(1280) }}{% endif %}" }
```

The theme writes, for every page: `<title>`, `<link rel="canonical">` with the absolute address of the page (list pages after the first keep `?page=N`), `<meta name="description">`, Open Graph (`og:type` is `article` on `match` pages and `website` on the others, plus title, description, image, URL, site name and locale) and Twitter Card tags (`summary_large_image` when there is an image). The description is collapsed to one line and cut at 200 characters; an image path is turned into an absolute address. A theme of your own reads the same values from `page.seo` (`canonical`, `description`, `image`, `noindex`, `type`, `json_ld`, `author`, `published`, `modified`). The site name and logo come from the published appearance when it sets them, otherwise from the organization; the visible header, `og:site_name` and the JSON-LD `publisher` always use the same values.

Every page also carries [JSON-LD](https://schema.org) in a `<script type="application/ld+json">`: `name`, `url`, `inLanguage`, `description` and `image` when the page has them, typed `WebSite` on the home page, `Article` on `match` pages (with `headline`, `mainEntityOfPage` and the organization as `publisher`, with its logo), `CollectionPage` on `list` pages and `WebPage` on the others. A page changes it with these `seo` keys:

| Key | Notes |
| --- | --- |
| `schema` | A schema.org type such as `Product`, `BlogPosting` or `Event` (letters and digits, starting with a capital letter). |
| `author` | A template expression with the public author's name. Adds `<meta name="author">` and a `Person` as JSON-LD `author`. |
| `published` | A template expression with the publication date (ISO 8601). Adds `article:published_time` and JSON-LD `datePublished`. |
| `modified` | A template expression with the last revision date (ISO 8601). Adds `article:modified_time` and JSON-LD `dateModified`; without it, `dateModified` repeats `published`. |
| `structured_data` | A template expression that renders a JSON object; its keys are added to the generated one and replace keys with the same name (`@context` is kept). Use the `tojson` filter for values. If it does not render a JSON object, the page keeps the generated data and the server logs a warning. |

Empty results are left out.

```toml
seo = { schema = "BlogPosting", author = "{{ record.byline or '' }}", published = "{{ record.published_at or '' }}", modified = "{{ record.revised_at or '' }}", structured_data = '{"keywords": {{ record.keywords | tojson }}}' }
```

- **Sitemap.** `/sitemap.xml` under the site address lists static pages, list pages and every public record of `match` pages, with `lastmod` from `updated_at`, up to 50 000 addresses. With more than one language, every address lists its translations as `<xhtml:link rel="alternate" hreflang>`, plus `x-default` for the main language. It is cached and refreshed with the pages it lists.
- **Breadcrumbs.** Pages below the home page carry a `BreadcrumbList` JSON-LD with the home page, each parent address that is itself a page, and the current page (`page.seo.breadcrumbs`).
- **robots.txt.** When the site is at the root of a host, `/robots.txt` allows everything except `/api/`, `/_aphel/app/` and the app path, and points to the sitemap.
- **Indexing switch.** Site → *Address and cache* → *Let search engines find the site*. Off, every page carries `noindex`, `robots.txt` disallows everything and the sitemap answers 404. The site stays open to anyone with the link.
- **Moved records.** When the field used by a `match` page changes (a new slug), the old address answers `301` to the new one. Chains collapse (`a → b → c` becomes `a → c`, `b → c`), and a record that takes an address back stops its redirect. Redirects only apply when no public record has the old value.
- **Redirects by hand.** Site → *Redirects* sends an old address (a path relative to the site address, such as `/old-page`) to a site path or to another website, with `301` or, for temporary moves, `302`. They apply only when no page answers the address, also under the language prefixes (`/en/old-page` goes to `/en/<target>`).

## Feeds

A list page with `feed = true` answers `<page address>/feed.xml` with RSS 2.0: the 20 newest records in the page's order, with `title` (or `name`), `link` and `guid` from the record's public address, `description` from `seo_description`, `summary` or `description`, `pubDate` from `published_at` (or `created_at`) and `dc:creator` from `byline`. Prefix and term pages give one feed per collection and term. Feeds follow the language prefix, are cached with the page and refreshed when a record of the entity changes. A theme of your own links them with `page.feed_url`.

## Public search

`/busca` (Portuguese sites) and `/search` show a search form and the published records whose public `title`, `name`, `summary`, `description` or `seo_description` contain the words typed (`?q=`, 2 to 100 characters), with links to their `match` pages: at most 10 per public entity and 30 in all. Entities without a `match` page are left out. The page extends the theme layout, is never cached and carries `noindex`; `site.search` holds its address, and the built-in theme shows it in the header.

## Images

With `images = true`, a template can show the images attached to a record:

```html
{% for image in record.images %}
  <img src="{{ image | image_url(960) }}" alt="{{ image.name }}">
{% endfor %}
```

Images are served under `/_aphel/site/files/` on any host of the installation. Every request checks that the record is still public (it exists, is not archived and meets `where`) and that the file is an image attached to it; otherwise it answers 404. Resized versions are generated on first use and kept in the file storage. Responses carry an `ETag` and `Cache-Control: public, max-age=86400`, so a browser may keep showing an image for up to a day after the record stops being public.

## Appearance and collection presentations

The Site appearance screen edits `site.appearance` (target `site`) and `site.presentation` (target `<collection entity>:<record id>`). The site owns both resources. Permissions are `site.appearance.edit` and `site.presentation.edit`; editing entries grants neither. Administrators receive both by default.

Appearance requires the optional `publication` module. Without it, the site renders theme defaults and the editor is unavailable. Drafts are separate from the protected public projection. Save draft, Apply appearance, Discard changes and Restore defaults use the record pipeline and shared publication operations. Applying can require approval. Restoring defaults changes the draft until it is applied and never restores an article. Optimistic versions protect concurrent edits.

Precedence is host defaults, active theme defaults, published site appearance, published collection presentation and permitted block options. Omit a property to restore inheritance. Collection overrides cannot change global identity. `GET /api/v1/site/appearance` returns properties, presets, available targets and theme metadata. `POST /api/v1/site/appearance/preview` renders public data with authorized appearance overrides; `empty: true` previews an empty collection. The response is authenticated, includes noindex metadata and is displayed in a sandboxed frame. Desktop and mobile previews use the public renderer. Editorial drafts use their separate authorized preview flow.

The built-in theme includes light, dark and device color modes, local system fonts, shared spacing and radius tokens, responsive navigation, lists/grids, a featured-first variant, card variants, cover placement and reading width. Editorial, Magazine and Minimal presets set the same properties. Text-block headings receive stable block-based anchors for the optional table of contents. Sharing uses ordinary links and the host's clipboard script. Cover focal points and captions belong to the entry snapshot; card ratio and fit belong to appearance.

Themes may declare `[site.theme.appearance.defaults]` with values from the appearance catalog and `properties = [...]` in `[site.theme.appearance]` to declare supported controls. An empty list supports the whole catalog. The editor identifies saved options not declared by a theme. Themes can replace `site/theme/appearance.html` and `site/theme/presentation_style.html`, or map namespaced public presentation templates to their own templates under `[site.theme.appearance.presentations]`. These overrides receive the public projection and do not change exposure filters or authorization. Fonts and scripts remain declared theme assets under the existing CSP rules. Theme selection is saved and published with global appearance.

The `site.appearance` provisioner seeds a global or collection resource with `{ target, values }`, validates it and publishes through `publication`. Existing resources are preserved. Pending publication produces an explicit failure instead of reporting readiness. The Blog pack seeds only its collection presentation, preserving the site's identity. Retries recognize the seed marker and resume incomplete publication; edited drafts are preserved.
