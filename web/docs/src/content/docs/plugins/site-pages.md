---
title: Site pages
description: Declare public pages (blog posts, catalogs, landing pages) that the site module renders on the server from your published records, with no backend code.
sidebar:
  order: 53
---

Site pages are public HTML pages rendered on the server by the `site` module. A plugin declares them in `plugin.toml` and ships the templates in its package. Aphel looks up the records, applies the publication filter, renders the template inside the site theme and answers with complete HTML, so search engines and link previews see the content. A plugin that only publishes pages needs no `backend.wasm`.

Use site pages for content that visitors read: posts, products, categories, portfolios. Use a [public web app](/_aphel/docs/plugins/public-web-apps/) for an interactive single-page tool.

The `site` module must be enabled. An administrator chooses where the site answers under Settings → General: its path (`/s` by default, or `/` when the app moves to a path such as `/app`) and, optionally, its own domain. Page paths in this guide are relative to that address.

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
| `seo` | table | Optional. `description` and `image` are template expressions; `noindex = true` keeps the page out of search engines and the sitemap. See [Search engines](#search-engines). |

A page without `entity` is static and has no parameters. A page with `entity` needs either `match` or `list`.

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
- Every response carries a content security policy that allows scripts, fonts and connections only from the same origin, and images from the same origin, `data:` and `https:`.
- Unknown paths, records that fail the publication conditions and list pages past the last one answer 404 with the theme's page.
- Rendered pages are kept in a cache in each server process. A change to a public record (create, update, delete, archive, restore, attachments) clears the pages that show it: its own page and the list pages of its entity. A change to the organization (name, color, settings) clears every page. Other server processes see the change within a few seconds. Plugin installs and upgrades start a new cache.
- Pages carry an `ETag` and `Cache-Control: public, max-age=60, stale-while-revalidate=600`, and answer `304` to a matching `If-None-Match`. Error pages are never cached.
- Renders (pages not in the cache) are limited to 120 per minute per client address; above that the site answers `429`.

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
- **Indexing switch.** Settings → General → *Let search engines find the site*. Off, every page carries `noindex`, `robots.txt` disallows everything and the sitemap answers 404. The site stays open to anyone with the link.
- **Moved records.** When the field used by a `match` page changes (a new slug), the old address answers `301` to the new one. Chains collapse (`a → b → c` becomes `a → c`, `b → c`), and a record that takes an address back stops its redirect. Redirects only apply when no public record has the old value.

## Images

With `images = true`, a template can show the images attached to a record:

```html
{% for image in record.images %}
  <img src="{{ image | image_url(960) }}" alt="{{ image.name }}">
{% endfor %}
```

Images are served under `/_aphel/site/files/` on any host of the installation. Every request checks that the record is still public (it exists, is not archived and meets `where`) and that the file is an image attached to it; otherwise it answers 404. Resized versions are generated on first use and kept in the file storage. Responses carry an `ETag` and `Cache-Control: public, max-age=86400`, so a browser may keep showing an image for up to a day after the record stops being public.
