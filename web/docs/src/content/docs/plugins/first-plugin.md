---
title: Your first plugin
description: Build, pack and install a complete plugin with a managed entity, a permission-protected route and a hook that rejects invalid data.
sidebar:
  order: 2
---

This tutorial builds a plugin called `library`. It adds a **Book** entity with four fields, normalizes and validates the ISBN with a JavaScript hook, and exposes a small route that counts books. You will pack it with the `aphel-plugin` command line tool, install it from the interface and from the API, and watch it work.

## Prerequisites

- A running Aphel installation and a user with the `system.manage` permission (the `admin` role has it).
- A checkout of the Aphel repository, to build the command line tool and to use the JavaScript SDK.
- Rust (to build the tool) and a recent Node.js with `npm` (for the JavaScript backend).

In the commands below, replace `/path/to/aphel` with the path of your checkout.

## 1. Build the command line tool

```bash
cargo build --release --manifest-path /path/to/aphel/sdk/cli/Cargo.toml
export PATH="/path/to/aphel/sdk/cli/target/release:$PATH"
```

The tool has three commands: `aphel-plugin new`, `aphel-plugin build` and `aphel-plugin pack`.

## 2. Create the project

```bash
aphel-plugin new library --js --sdk /path/to/aphel/sdk/js/aphel-plugin-sdk
cd aphel-library
```

`new` creates the directory `aphel-library` with `plugin.toml`, `package.json`, `.gitignore` and `src/plugin.js`. The `--sdk` option is written into `package.json` as a `file:` dependency on the SDK. Without it the tool assumes the SDK is at `../aphel/sdk/js/aphel-plugin-sdk` relative to the new project.

## 3. Write the manifest

Replace the content of `plugin.toml`.

```toml
key = "library"
name = "Library"
version = "0.1.0"
description = "Keeps track of the books you want to read."
api = 1
requires_aphel = ">=0.1.0"

[[entities]]
key = "book"
label = "Book"
label_plural = "Books"
storage = "managed"
permission_scope = "organization"

[[entities.fields]]
key = "title"
label = "Title"
type = "text"
required = true
searchable = true
max_length = 200

[[entities.fields]]
key = "isbn"
label = "ISBN"
type = "text"
unique = true

[[entities.fields]]
key = "pages"
label = "Pages"
type = "number"
min = 1
decimals = 0

[[entities.fields]]
key = "status"
label = "Status"
type = "select"
options = [
  { value = "to_read", label = "To read" },
  { value = "reading", label = "Reading" },
  { value = "done", label = "Done" },
]

[[hooks.subscribe]]
hook = "record.saving"
record_types = ["library.book"]

[[hooks.subscribe]]
hook = "record.validating"
record_types = ["library.book"]

[[routes]]
method = "GET"
path = "/summary"
permission = "library.book.read"
```

What each part does:

- `key`, `name`, `version` and `api = 1` identify the plugin. The key is also the prefix of everything the plugin owns, so `library.book` is the record type. Keys of plugins that declare entities may use only lowercase letters and underscores.
- `storage = "managed"` makes Aphel create a table with real columns for the declared fields. `permission_scope = "organization"` sets the scope of the permissions granted by default to the system roles `manager` and `member`; the `admin` role always has everything.
- The four `[[entities.fields]]` tables define the columns. `isbn` is `unique`, so two books cannot share an ISBN.
- The two `[[hooks.subscribe]]` tables register the backend for `record.saving` (which may change values before they are written) and `record.validating` (which may veto the write).
- The route needs `library.book.read`, a permission that exists because the plugin declares the `book` entity.

Every key and limit is described in [Manifest](/docs/plugins/manifest/).

## 4. Write the backend

Replace the content of `src/plugin.js`.

```js
import { definePlugin, hooks, ok, PluginError } from '@aphel/plugin-sdk'

const BOOK = 'library.book'
const ISBN_PATTERN = /^(\d{9}[\dXx]|\d{13})$/

function compactIsbn(value) {
  return value.replace(/[\s-]/g, '')
}

export const { onHook, onEvent, onRoute, onSchedule, onAction, onWidget, onOperation, onContribution } =
  definePlugin({
    onHook(ctx, call) {
      if (call.is(hooks.SAVING, BOOK) && typeof call.values.isbn === 'string') {
        return { values: { isbn: compactIsbn(call.values.isbn) } }
      }
      if (call.is(hooks.VALIDATING, BOOK)) {
        const isbn = call.values.isbn
        if (typeof isbn === 'string' && isbn !== '' && !ISBN_PATTERN.test(isbn)) {
          throw PluginError.field('invalid_isbn', 'isbn', 'An ISBN has 10 or 13 digits.')
        }
      }
      return {}
    },

    onRoute(ctx, request) {
      if (request.is('GET', '/summary')) {
        const { total } = ctx.records.list(BOOK, { perPage: 1 })
        return ok({ total })
      }
    },
  })
```

Three things to notice:

- The `record.saving` branch returns a patch of values. Aphel merges it into the record before writing, so `978-0-441-17271-9` is stored as `9780441172719`. A patch may only name fields that exist and are editable, otherwise the write fails with `plugin_field_not_writable`.
- The `record.validating` branch throws a `PluginError`. Throwing from a veto hook cancels the write and rolls it back. `PluginError.field` attaches the error to the `isbn` field and the caller receives a `422` with the code `invalid_isbn`.
- The route returns plain JSON with `ok`. `request.is` compares the HTTP method and the route exactly as declared in the manifest.

The file must export the eight handlers as shown, even when you implement only some of them.

## 5. Build and pack

```bash
aphel-plugin pack
```

`pack` installs the npm dependencies when `node_modules` is missing, runs the `build:backend` script (which bundles `src/plugin.js` and compiles it to `backend.wasm`), and writes `library-0.1.0.aphelplugin` next to `plugin.toml`. The first build can take a while because the JavaScript runtime is compiled into the component.

Check the content of the package.

```bash
unzip -l library-0.1.0.aphelplugin
```

You should see `plugin.toml` and `backend.wasm`. The plugin has no web bundle, so there are no `web/` entries.

## 6. Install from the interface

1. Sign in as a user with `system.manage`.
2. Open **Settings**, then **Integrations**, then **Plugins**.
3. Choose **Install plugin** and select `library-0.1.0.aphelplugin`.
4. Aphel runs a dry run and shows the consent dialog: the entity `library.book`, the two hooks, the route, the unique field and the permissions the plugin uses. Nothing is installed yet.
5. Confirm. The plugin is installed and enabled, and the page reloads.

## 7. Install from the API

The dry run is the same call with `dry_run=true`. It answers `200` and installs nothing.

```bash
curl -sS -X POST "$APHEL_URL/api/v1/wasm-runtime?dry_run=true" \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -F "package=@library-0.1.0.aphelplugin"
```

The real installation answers `201` with the plugin as stored.

```bash
curl -sS -X POST "$APHEL_URL/api/v1/wasm-runtime" \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -F "package=@library-0.1.0.aphelplugin"
```

The token must belong to a user with `system.manage` and carry that scope. To see the plugin afterwards:

```bash
curl -sS "$APHEL_URL/api/v1/wasm-runtime" -H "Authorization: Bearer $APHEL_TOKEN"
```

If the response contains `backend_error`, the backend did not load. See [Testing and debugging](/docs/plugins/testing-and-debugging/).

## 8. See it working

Create a book through the generic record API. The token needs the `library.book.create` permission.

```bash
curl -sS -X POST "$APHEL_URL/api/v1/entities/library.book/records" \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"title":"Dune","isbn":"978-0-441-17271-9","pages":688,"status":"to_read"}'
```

The response has `"isbn":"9780441172719"`: the `record.saving` hook removed the hyphens.

Now send an invalid ISBN.

```bash
curl -sS -X POST "$APHEL_URL/api/v1/entities/library.book/records" \
  -H "Authorization: Bearer $APHEL_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"title":"Broken","isbn":"123","status":"to_read"}'
```

The answer is `422` with `error.code` equal to `invalid_isbn` and the message `An ISBN has 10 or 13 digits.`; the record is not created. The same rule applies when a person saves a book in the interface, because hooks sit in the record pipeline, not in the API.

Call the route. Private routes live under `/api/v1/p/<plugin key>/`.

```bash
curl -sS "$APHEL_URL/api/v1/p/library/summary" -H "Authorization: Bearer $APHEL_TOKEN"
```

The answer is `{"total":1}`.

The system roles `manager` and `member` received the `library.book.*` permissions with organization scope when the plugin was installed (`member` does not get `export`). To change who can do what, edit the roles as described in [Roles and permissions](/docs/admin/roles-and-permissions/).

## 9. Upgrade

Change something, raise `version` to `0.1.1` in `plugin.toml`, run `aphel-plugin pack` again and install the new package the same way. Installing a package with the same key is an upgrade: fields you stopped declaring are deactivated (their data is kept), a changed field type is refused with `plugin_field_type_changed`, and settings and stored data are kept. The details are in [Lifecycle](/docs/plugins/lifecycle/).

To remove the plugin, use **Uninstall** on the Plugins page or `DELETE /api/v1/wasm-runtime/library`. Add `?purge=true` to delete the `library.book` records and the table as well.

## Next steps

- Add a Rust backend instead: [Rust SDK](/docs/plugins/rust-sdk/).
- React to events and schedule work: [Events](/docs/plugins/events/), [Schedules](/docs/plugins/schedules/).
- Let admins configure the plugin: [Settings and secrets](/docs/plugins/settings-and-secrets/).
- Add screens: [Web bundle](/docs/plugins/web-bundle/).
