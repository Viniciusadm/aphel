---
title: Site pages
description: Declare public pages (blog posts, catalogs, landing pages) that the site module renders on the server from your published records, with no backend code.
sidebar:
  order: 53
---

Site pages are public HTML pages rendered on the server by the `site` module. A plugin declares them in `plugin.toml` and ships the templates in its package. Aphel looks up the records, applies the publication filter, renders the template inside the site theme and answers with complete HTML, so search engines and link previews see the content. A plugin that only publishes pages needs no `backend.wasm`.

Use site pages for content that visitors read: posts, products, categories, portfolios. Use a [public web app](/_aphel/docs/plugins/public-web-apps/) for an interactive single-page tool.

The `site` module must be enabled. An administrator chooses where the site answers under Settings → Site: its path (`/s` by default, or `/` when the app moves to a path such as `/app`) and, optionally, its own domain. Page paths in this guide are relative to that address.

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
| `images` | boolean | When `true`, image attachments of a public record (JPEG, PNG, WebP, GIF) become public too and reach templates as `images`. Default `false`. |
| `translated` | array of strings | Public fields that have versions per language. See [Languages](#languages). |

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
| `path` | string | Starts with `/`. Segments use lowercase letters, digits, `-`, `_` and `.`, and cannot start with `_`. A parameter looks like `{slug}`. `/sitemap.xml` and `/robots.txt` are reserved. |
| `template` | string | A file of the package under `site/`, ending in `.html`. |
| `entity` | string | Optional. An entity listed in `[[site.public]]`. |
| `match` | string | One record: the public field compared with the only path parameter, which must have the same name (`/blog/{slug}` with `match = "slug"`). |
| `list` | table | A paginated list: `sort` (public fields, `-` for descending, comma separated) and `per_page` (1 to 50, default 10). The page reads `?page=N`. |
| `title` | string | Optional. A template expression rendered with the page context, used in `<title>` and in the share previews. |
| `data` | string | Optional. Path of a GET route of the plugin declared with `site = true`, using the page parameters (`/products/{slug}`). Its JSON answer reaches the template as `data`; a 404 answer makes the page 404. See [Dynamic pages](#dynamic-pages). |
| `seo` | table | Optional. `description` and `image` are template expressions; `noindex = true` keeps the page out of search engines and the sitemap. See [Search engines](#search-engines). |

A page without `entity` is static and has no parameters, unless it declares `data`. A page with `entity` needs either `match` or `list`.

Two plugins cannot serve the same path shape: installing the second fails with `409 site_route_conflict`. Paths are matched from the most specific to the least specific, so `/blog/archive` wins over `/blog/{slug}`.

## Write the templates

Templates use [MiniJinja](https://docs.rs/minijinja), a Jinja2 dialect. Output is HTML-escaped by default. Extend the site theme with `{% extends "layout.html" %}` and fill `{% block content %}`; the theme also has `title` and `head` blocks. Include your own partials by their package path: `{% include "site/_card.html" %}`.

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
| `record` | The record of a `match` page, with the public fields only, plus `images` when the entity exposes them: a list of `{ id, name, type, url }`, at most 20. |
| `records`, `pagination` | The records of a `list` page and `page`, `pages`, `total`, `previous_url`, `next_url`. |

| Function or filter | Use |
| --- | --- |
| `url_for(key, **params)` | Address of a page of the same plugin, including the site path. |
| `value \| markdown` | Renders Markdown and removes unsafe HTML (scripts, event handlers). |
| `value \| date(format)` | Formats an ISO date or date-time. Default `%d/%m/%Y`. |
| `image \| image_url(width)` | Address of an image, resized to the smallest of 320, 640, 960, 1280 or 1920 pixels that is at least `width` wide. Without `width`, the original file. |

Templates are compiled when the plugin is installed: a syntax error, a missing template or a page that breaks the rules above fails the install with `422 plugin_site_invalid`, listing every problem. Templates cannot reach the network, files or the database: they only see the variables above. A render that runs too long stops and the visitor sees the theme's error page.

## How pages are served

- Only `GET` and `HEAD` are answered.
- Every response carries a content security policy that allows scripts, fonts and connections only from the same origin (plus the import map, by hash), images from the same origin, `data:` and `https:`, and the origins the page's plugin and the active theme declare in `[site.csp]`.
- Unknown paths, records that fail the publication conditions and list pages past the last one answer 404 with the theme's page.
- Rendered pages are kept in a cache in each server process. A change to a public record (create, update, delete, archive, restore, attachments) clears the pages that show it: its own page and the list pages of its entity. A change to the organization (name, color, settings) clears every page. Other server processes see the change within a few seconds. Plugin installs and upgrades start a new cache.
- Pages carry an `ETag` and `Cache-Control: public, max-age=60, stale-while-revalidate=600`, and answer `304` to a matching `If-None-Match`. Error pages are never cached.
- Renders (pages not in the cache) are limited to 120 per minute per client address; above that the site answers `429`.

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

An island is an ES module in the package at `site/islands/<name>.js` (lowercase letters, digits, `-` and `_`; at most 30). A template places it with `island`:

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

`react`, `react/jsx-runtime`, `react-dom`, `react-dom/client` and `@aphel/ui` resolve through the site's import map to the same files the application uses; anything else must be bundled into the island. An island only exists in pages of its own plugin; asking for an unknown island fails the render.

### Content security policy

```toml
[site.csp]
script = ["https://js.stripe.com"]
connect = ["https://api.stripe.com"]
frame = ["https://js.stripe.com"]
```

`script`, `style`, `img`, `font`, `connect` and `frame` accept up to 10 origins each, written as `https://host`, `https://host:port` or `https://*.domain`. They apply to the pages of the plugin that declares them, and to every page when the plugin is the active theme. The consent dialog lists them.

## Customer accounts

Visitors can have an account on the site, separate from the people who use Aphel. Accounts turn on by themselves when an installed plugin has customer pages, and an administrator can also turn them on in Settings → Site.

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

Customers are the record type `site.customer` (name, email, confirmation and last sign-in dates), readable with the permission `site.customers.read` and listed in Settings → Site. Account creation, changes and closing publish `site.customer.created|updated|deleted` with an audit entry. Closing an account, or a privacy request for it, erases the name, email and password and signs the customer out everywhere.

## Menu

```toml
[[site.menu]]
key = "blog"
label = "Blog"
page = "index"
```

Each item points to a page of the same plugin that has no parameters; at most 20 items per plugin. `labels = { en = "Blog", es = "Blog" }` gives the label per site language; without one, `label` is used. Labels are up to 60 characters. In Settings → Site the organization reorders the items of every plugin and hides the ones it does not want; new items appear at the end. Templates read the menu as `site.menu`, a list of `{ label, url, active }` (`active` when the visitor is on that page or below it).

The theme also receives `site.tagline` and `site.logo_url` (the organization logo, served at `/_aphel/site/logo`).

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

Include `{% include "aphel/head.html" %}` in the `<head>` of the layout: it writes the canonical, description, Open Graph and Twitter tags, the import map and the island loader. Partials of the theme are included by their package path (`{% include "site/theme/_header.html" %}`). The organization picks the theme in Settings → Site; only one is active, and the built-in theme is used when none is chosen or the chosen plugin is disabled. A theme plugin needs no entities or pages.

## Languages

The site speaks the organization language. In Settings → Site an administrator adds up to five more (Portuguese, English, Spanish, French, German, Italian). Each extra language answers under its code after the site address: `/s/en/blog`, `/s/es/blog`. The default language keeps the addresses without a code.

For each request in an extra language:

- `site.language` (and `site.locale`, used in `<html lang>`) is the language code; `site.prefix`, `site.home`, `url_for`, pagination, the menu and the account pages carry the language code, so links stay in the language.
- Fields listed in `translated` take the value of `<field>_<code>` when the record has it filled (`title_en`, `body_pt_br`; the code is lowercase with `-` replaced by `_`). Otherwise the original value is used. Declare the variant fields in the entity; they do not need to be listed in `fields`.
- Menu and *My account* labels come from `labels`.
- Theme texts and account pages are in Portuguese for `pt*` languages and in English for the others.

`aphel/head.html` writes `<link rel="alternate" hreflang>` for every language plus `x-default`, and `site.alternates` lists `{ code, url, path, current }` for a language switcher (the built-in theme shows one in the header). The sitemap lists every page in every language. Values used in `match` (a slug) are shared by all languages.

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

The theme writes, for every page: `<title>`, `<link rel="canonical">` with the absolute address of the page (list pages after the first keep `?page=N`), `<meta name="description">`, Open Graph (`og:type` is `article` on `match` pages and `website` on the others, plus title, description, image, URL, site name and locale) and Twitter Card tags (`summary_large_image` when there is an image). The description is collapsed to one line and cut at 200 characters; an image path is turned into an absolute address. A theme of your own reads the same values from `page.seo` (`canonical`, `description`, `image`, `noindex`, `type`).

- **Sitemap.** `/sitemap.xml` under the site address lists static pages, list pages and every public record of `match` pages, with `lastmod` from `updated_at`, up to 50 000 addresses. It is cached and refreshed with the pages it lists.
- **robots.txt.** When the site is at the root of a host, `/robots.txt` allows everything except `/api/`, `/_aphel/app/` and the app path, and points to the sitemap.
- **Indexing switch.** Settings → Site → *Let search engines find the site*. Off, every page carries `noindex`, `robots.txt` disallows everything and the sitemap answers 404. The site stays open to anyone with the link.
- **Moved records.** When the field used by a `match` page changes (a new slug), the old address answers `301` to the new one. Chains collapse (`a → b → c` becomes `a → c`, `b → c`), and a record that takes an address back stops its redirect. Redirects only apply when no public record has the old value.

## Images

With `images = true`, a template can show the images attached to a record:

```html
{% for image in record.images %}
  <img src="{{ image | image_url(960) }}" alt="{{ image.name }}">
{% endfor %}
```

Images are served under `/_aphel/site/files/` on any host of the installation. Every request checks that the record is still public (it exists, is not archived and meets `where`) and that the file is an image attached to it; otherwise it answers 404. Resized versions are generated on first use and kept in the file storage. Responses carry an `ETag` and `Cache-Control: public, max-age=86400`, so a browser may keep showing an image for up to a day after the record stops being public.
