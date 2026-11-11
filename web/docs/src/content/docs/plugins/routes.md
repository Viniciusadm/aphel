---
title: Routes
description: Expose private and public HTTP endpoints from a plugin, with request and response shapes, permissions and rate limits.
sidebar:
  order: 8
---

A plugin can answer HTTP requests. Routes are declared in the manifest and handled by the backend's `on-route`. They are useful for actions that do not fit the record API: reports, imports, webhooks from other services, downloads, and endpoints for the plugin's own web screens.

## Declaring routes

```toml
[[routes]]
method = "GET"
path = "/books/{id}/card"
permission = "library.book.read"

[[routes]]
method = "POST"
path = "/webhooks/stock"
public = true
```

| Key | Description |
| --- | --- |
| `method` | `GET`, `POST`, `PUT`, `PATCH` or `DELETE`. Uppercase. |
| `path` | Starts with `/`, no `..`. A segment written `{name}` matches one path segment and is passed to the backend as a parameter. |
| `permission` | Permission the caller must hold. Not allowed with `public`. |
| `public` | `true` for a route that does not need sign-in. |
| `site` | `true` for a route called by islands on the public site, at `/_aphel/site/api/<plugin key>/<path>`. See [Site pages](/_aphel/docs/plugins/site-pages/#site-routes). |

A plugin with routes needs a backend.

## URLs

| Kind | URL |
| --- | --- |
| Private route | `/api/v1/p/<plugin key>/<path>` |
| Public route | `/api/v1/public/p/<plugin key>/<path>` |

For the routes above, the private one is `GET /api/v1/p/library/books/0192.../card`. The public one is `POST /api/v1/public/p/library/webhooks/stock`.

Matching rules:

- Private and public routes are matched separately. A public route is not reachable under the private prefix and the other way round.
- Method and path must both match. The first declared route that matches wins, so declare specific paths before routes with parameters at the same position.
- The number of segments must be equal. Trailing slashes are ignored.
- No match, or a plugin that is not installed or is disabled, answers `404`: `plugin_route_not_found` or `plugin_not_found`.
- Methods other than the five above are not routable.

## Private routes

Private routes need an authenticated caller: a session cookie or an API token in `Authorization: Bearer`. Non-GET requests with a session cookie must also send `X-Requested-With: aphel`, like every Aphel endpoint; token calls do not need it.

1. If the route declares `permission`, the caller must hold it. Otherwise the answer is `403`. The check uses the caller's real permissions, before the plugin's capabilities are applied.
2. The backend runs with the intersection of the caller's permissions and the plugin's capabilities, and with the caller's record scope. Writes made by the route are attributed to the caller.

A private route without `permission` can be called by any signed-in user.

## Public routes

Public routes need no sign-in. There is no caller, so the backend runs as the plugin, with the permissions of the user who installed it limited to the plugin's capabilities. If that user is no longer an active member, public routes answer `503 plugin_owner_inactive`.

Public routes are rate limited per minute: 60 requests per plugin and client address, and 600 per plugin. Beyond that the answer is `429`. The actor of the request has the kind `anonymous`.

Treat every input of a public route as hostile: validate it, authenticate the sender yourself (for example with a shared secret stored in a [secret setting](/_aphel/docs/plugins/settings-and-secrets/) and compared in the handler), and keep in mind that the plugin's installer permissions are behind the route.

## The request

The backend receives:

| Field | JavaScript | Rust | Description |
| --- | --- | --- | --- |
| `method` | `request.method` | `request.method` | `GET`, `POST`, ... |
| `path` | `request.path` | `request.path` | The matched path with a leading `/`, for example `/books/0192.../card`. |
| `route` | `request.route` | `request.route` | The declared pattern, for example `/books/{id}/card`. Use it to dispatch. |
| `params` | `request.params` | `request.params` | Map of path parameters, all strings. |
| `query` | `request.query` | `request.query` | Map of query string values, all strings. When a name repeats, the last value wins. |
| `headers` | `request.headers` | `request.headers` | List of `[name, value]` pairs with lowercase names. `authorization`, `cookie` and `proxy-authorization` are never forwarded. |
| `body` | `request.body` | `request.body` | The parsed JSON body, or `null` when the body is empty. |
| `actor` | `request.actor` | `request.actor` | Who is calling: `{ kind, id, user_id }`. |

The body must be valid JSON. Anything else fails before your code runs with `422 validation_error`: `the body must be JSON`. The content type is not checked.

## The response

Return a status and a JSON body. The body is sent as it is, without the `{ "data": ... }` envelope used by Aphel's own endpoints.

```js
onRoute(ctx, request) {
  if (request.is('GET', '/books/{id}/card')) {
    const book = ctx.records.get('library.book', request.params.id)
    if (!book) throw PluginError.notFound('book_not_found', 'No such book.')
    return ok({ title: book.title, status: book.status })
  }
  if (request.is('POST', '/webhooks/stock')) {
    const created = ctx.records.create('library.book', request.body)
    return respond(201, { id: created.id })
  }
}
```

| Case | Result |
| --- | --- |
| Handler returns `{ status, body }` | That status (when it is a valid HTTP status; otherwise `200`) and the body as JSON. |
| Handler throws a plugin error | Status and body as described in [Hooks](/_aphel/docs/plugins/hooks/#rejecting-with-errors), in Aphel's error envelope `{ "error": { "code", "message", "details" } }`. |
| JavaScript handler returns nothing | `404 route_not_found`, so a handler can implement only some routes. |
| Timeout (10 seconds), trap, out of memory | `503 plugin_failed`. |

In Rust the default `on_route` returns `route_not_found` with status 404.

### Downloads and raw bodies

To send a file, call `respond-bytes` (JavaScript `ctx.respondBytes(contentType, filename, body)`) before returning. The JSON body of the handler's response is then ignored; the status is still used.

| Rule | Value |
| --- | --- |
| Allowed content types | `text/csv`, `text/plain`, `text/calendar`, `application/pdf`, `application/json`, `application/zip`, `application/octet-stream`, `image/png`, `image/jpeg` (parameters such as `charset` are accepted) |
| Maximum size | 10 MiB (`413 plugin_raw_body_too_large`) |
| Not an allowed type | `422 plugin_raw_body_invalid` |
| Called outside a route | `409 plugin_raw_body_outside_route` |
| `filename` | Optional. Only letters, digits, `.`, `-` and `_` are kept, at most 120 characters. When set, the response is an attachment. |
| Headers added | `X-Content-Type-Options: nosniff`, `Cache-Control: private, no-store` |

## What a route may do

A route runs outside any transaction and is not retried, so it may call every host function directly: write records, use key-value storage, send notifications, emit events and fetch URLs from allowed hosts. It has ten seconds. Use `defer-effect` when an action must survive a failure after the response.

## Routes for built-in paths

When a plugin replaces a built-in module through `provides.contracts`, the `[aliases]` table can map the module's API paths to routes of the plugin. A request to the built-in path is then answered by the route. Aliases are described in [Lifecycle](/_aphel/docs/plugins/lifecycle/).
