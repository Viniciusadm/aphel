---
title: Public web apps
description: Ship a static web app with a plugin and serve it at /p/<key>/, for pages that visitors open without the Aphel interface.
sidebar:
  order: 54
---

A public web app is a folder of static files in your package that Aphel serves at `/p/<key>/`. Use it for pages that should not look like part of the Aphel interface: a customer-facing form, a status page, a booking page or a small tool. It is separate from the [web bundle](/_aphel/docs/plugins/web-bundle/), which plugs screens into the signed-in application. For content that visitors read and search engines index (posts, products), use [site pages](/_aphel/docs/plugins/site-pages/) instead: they are rendered on the server from your records.

## Declaring it

```toml
[web]
entry = "web/index.js"
public = "web/app"
```

`public` is a directory inside the package. The package reader requires it to start with `web/` and to contain `index.html`. Otherwise the install fails with `plugin_package_invalid` ("the public web app needs web/app/index.html, inside web/"). A plugin can have only a public app: the `entry` file must still exist, because it is checked on install. A one-line module such as `export default { key: 'my_plugin' }` is enough, and the bundle loads into the application as an empty module.

The `aphel-plugin` CLI packs only the contents of `web/dist/`. To produce `web/app/index.html` in the package, your build must write the app to `web/dist/app/`.

## How it is served

The app lives inside the internal app's base path, which is `/` by default and can be moved to a segment such as `/app` (see [Configuration](/_aphel/docs/admin/configuration/)). The paths below are relative to that base path: with the app at `/app`, `/p/<key>/` is `/app/p/<key>/`.

When the `site` module is enabled, the same paths also answer under the site address (`https://www.example.com/p/<key>/` when the site has its own domain), so visitors stay on the company's address. Aphel's own public pages work the same way (`/book/<slug>` for booking links, `/quote/<token>` for quotes), and the links Aphel sends point to the site address. Site pages cannot use these first segments (`p`, `book`, `quote`): the install fails with `site_route_conflict`.

| Request | Response |
| --- | --- |
| `GET /p/<key>` | Permanent redirect to `<key>/` (relative, so it keeps the base path). |
| `GET /p/<key>/` | `index.html`. |
| `GET /p/<key>/<path>` | The file `<public>/<path>`. |
| A path without a file extension that has no file | `index.html`, so client-side routing works. |
| A path with an extension that has no file | 404. |

- The app is served only while the plugin is enabled. Otherwise every path returns 404 (`plugin_not_found`).
- It is not authenticated by Aphel: anyone who can reach the server can open it.
- `index.html` is sent with `Cache-Control: no-cache`; other files with `public, max-age=3600`.
- Every response carries a strict content security policy: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'`, plus `X-Content-Type-Options: nosniff` and `Referrer-Policy: same-origin`.

The policy has consequences for your code:

- Scripts must be external files from the same origin. Inline `<script>` blocks, `eval` and scripts from other hosts do not run.
- Styles may be inline.
- Images can come from the same origin, `data:` URLs and any `https:` host. Fonts must be same-origin.
- `fetch` and `XMLHttpRequest` can reach only the same origin.
- The page cannot be embedded in a frame.

The consent dialog lists `GET /p/<key>/` among the public routes of the plugin.

## Trust

The app is served from the same origin as Aphel. Its JavaScript therefore runs with the origin of the application: the browser sends the session cookie of a signed-in user with every request to `/api`, and the code can call any API that user may call. Treat a public app as application code. It needs the same version acknowledgement as a web bundle (`acknowledge_web_version`) and the same level of trust. See [Security model](/_aphel/docs/plugins/security-model/).

## Linking the stylesheet

To follow the colors of the chosen interface, link the stylesheet endpoint of the organization default:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Status</title>
    <link rel="stylesheet" href="/api/v1/public/ui/default.css" />
    <link rel="stylesheet" href="./styles.css" />
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="./main.js"></script>
  </body>
</html>
```

`default.css` is public and returns the tokens of the organization's default interface plugin, or an empty file. It defines custom properties only, so your own stylesheet should read them with fallbacks, because the file is empty when no interface plugin is the default:

```css
body {
  margin: 0;
  background: var(--background, #ffffff);
  color: var(--foreground, #111111);
  font-family: var(--font-family-sans, system-ui, sans-serif);
}
```

The default token values themselves come from the stylesheet of the application, which a public app does not load. To use the full token set and the component styles, also link `/shared/ui.css`, which includes the base reset, the default tokens and the styles of the [UI library](/_aphel/docs/plugins/ui-library/) components.

Use relative URLs (`./main.js`) in `index.html`, because the app lives under `<base path>/p/<key>/`. The shared stylesheet of the UI library is at `/_aphel/shared/ui.css` whatever the base path.

## Calling APIs

The page can call the REST API on the same origin. Two kinds of endpoints are relevant:

| Endpoint | Auth | Notes |
| --- | --- | --- |
| `/api/v1/public/p/<key>/<path>` | None | Routes of your plugin declared with `public = true`. Rate limited to 60 requests per minute per client address and 600 per minute per plugin. They run with the identity of the user who installed the plugin, limited by the plugin capabilities. |
| `/api/v1/p/<key>/<path>` | Session | Private routes of your plugin. Anonymous visitors get 401. |
| Other `/api/v1/...` endpoints | Session or bearer token | The browser sends the session cookie. Requests that change data need the header `X-Requested-With: aphel`. |

Public endpoints under `/api/v1/public/` do not need the `X-Requested-With` header. Define the routes in the manifest and handle them in the backend as described in [Routes](/_aphel/docs/plugins/routes/).

```js
const response = await fetch('/api/v1/public/p/status_page/summary')
const { data } = await response.json()
```

## Packaging a public app

A Vite project for a public app can build into the right directory with `base: './'`:

```ts
import { defineConfig } from 'vite'

export default defineConfig({
  base: './',
  build: {
    outDir: 'dist/app',
    emptyOutDir: true,
  },
})
```

With `public = "web/app"` in the manifest, the CLI packs `web/dist/app/index.html` as `web/app/index.html`. The `entry` file (`web/dist/index.js`) must exist too. See [Packaging](/_aphel/docs/plugins/packaging/).
