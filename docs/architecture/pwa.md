# Installable app (PWA) and push notifications

Aphel can be installed from the browser on Android, iOS and desktop, without an app store. The same `web/` build embedded in the binary ships the manifest, the icons and the service worker; the backend sends Web Push notifications with the installation's VAPID keys.

## Installation

| File | What it is |
| --- | --- |
| `web/public/manifest.webmanifest` | Name `Aphel`, `display: standalone`, `start_url`/`scope` = `/`, colors and icons |
| `web/public/icon-192.png`, `icon-512.png` | `any` icons (generated from `favicon.svg`) |
| `web/public/icon-maskable-192.png`, `icon-maskable-512.png` | `maskable` icons (full background, logo in the safe area) |
| `web/public/apple-touch-icon.png` | 180×180 iOS Home Screen icon |
| `web/index.html` | `<link rel="manifest">`, `apple-touch-icon`, `theme-color` and `apple-mobile-web-app-*` meta tags |

The PNGs were generated with `rsvg-convert` from `favicon.svg` (and from a variant with a full background and the logo scaled down to 62% for the maskable icon and 80% for the Apple icon). To change the branding, regenerate the five files at the same sizes.

On Android/desktop the browser offers installation on its own; when the `beforeinstallprompt` event fires, the notification preferences screen shows the "Install Aphel" button (`web/src/pwa/install-store.ts`). On iOS, installation is always through Share → Add to Home Screen.

## Service worker

- **Source:** `web/scripts/pwa/service-worker.js`. The `serviceWorkerPlugin()` plugin (`web/scripts/pwa/vite-plugin.ts`, registered in `vite.config.ts`) runs only on `vite build`: it emits `dist/sw.js` with a version (hash of the bundle's file names) and the precache list (`/`, the entry chunk with its static imports and the CSS). That way every binary with new assets has a different `sw.js`, and the browser detects the update.
- **Plugins:** `/api/*` is never cached, with two exceptions that let the app start offline with its plugins: the list `/api/v1/public/wasm-runtime` (network first, cached copy when offline) and the assets under `/api/v1/public/wasm-runtime/<key>/` (cache first when the URL has the checksum in `?v=`, network first otherwise). They live in the `aphel-plugins` cache, which survives new service workers; each list refresh removes assets of plugins that left or changed version. Public plugin apps (`/p/*`) are not touched by the service worker.
- **Registration:** `registerPwa()` (`web/src/pwa/register.ts`, called in `main.tsx`) registers `/sw.js` with scope `/` **only in production** (`import.meta.env.PROD`). Under `vite dev` nothing is registered and there is no `sw.js`.
- **What is cached** (one `aphel-shell-<version>` cache; caches from old versions are deleted on activation):
  - navigation (`request.mode === 'navigate'`): network first; the HTML response becomes the shell stored at `/` and is used as the offline fallback;
  - `/assets/*` (files with a hash in the name): cache first;
  - **never** `/api/*`, `/health` or requests that are not `GET` or that come from another origin. No response with personal data goes into the service worker cache (offline data lives in IndexedDB, see [Offline use](#offline-use)).
- **Backend headers** (`back/kernel/src/api/web.rs`): `sw.js` is served with `Cache-Control: no-cache`, `Content-Type: text/javascript` and `Service-Worker-Allowed: /`; `manifest.webmanifest` with `application/manifest+json` and `no-cache`; `/assets/*` with `max-age=31536000, immutable`.

### New version notice

When a new binary is deployed, `sw.js` changes, the browser installs the new service worker and it stays **waiting** (`install` does not call `skipWaiting`). `web/src/pwa/update-store.ts` watches the registration (`updatefound`/`statechange`, a check every hour and when returning to the tab) and the `PwaShell` shows the "A new version of Aphel is available" notice with **Update** (sends `SKIP_WAITING` to the service worker and reloads on `controllerchange`) and **Later**.

## Web Push

### VAPID keys

- ECDSA P-256 key pair of the **installation** (a browser subscription only accepts one key).
- Generated the first time it is needed (`VaultVapidKeys`, `back/kernel/src/infrastructure/notifications/push/keys.rs`) and stored encrypted with `APP_ENCRYPTION_KEY` by the `SecretVault` in `installation_secrets` (`owner_type = "web_push"`, `owner_id = "vapid"`, `name = "private_key_pkcs8"`). The `SecretVault` gained `reveal_installation`/`store_installation_if_absent` for installation secrets; `aphel secrets rotate` also re-encrypts this table.
- Only the public key is exposed through the API (`GET /api/v1/notifications/push/public-key`). Without `APP_ENCRYPTION_KEY` the push routes respond 503 `configuration_error` and the worker does not register the delivery job.
- `sub` of the VAPID JWT: the public address saved in Settings → General when it is `https://`, otherwise `mailto:webpush@aphel.invalid`. In production use a public address with https (Apple's service rejects an invalid `sub`).

### Subscriptions (`push_subscriptions`)

One row per device/browser: `user_id`, `session_id`, `endpoint` (unique by SHA-256 `endpoint_hash`), `p256dh`, `auth`, `user_agent`, `created_at`, `updated_at`, `last_used_at`. It is identity data, like `sessions`.

| Route | What it does |
| --- | --- |
| `GET /api/v1/notifications/push/public-key` | VAPID public key (base64url) for `applicationServerKey` |
| `POST /api/v1/notifications/push/subscriptions` | `{ endpoint, keys: { p256dh, auth } }` → 204. Resubscribing the same endpoint moves the row to the current user and session |
| `POST /api/v1/notifications/push/subscriptions/unsubscribe` | `{ endpoint }` → 204 (idempotent) |

The endpoint must be `https` from a known push service (`fcm.googleapis.com`, `android.googleapis.com`, `*.push.services.mozilla.com`, `*.push.apple.com`, `*.notify.windows.com`); this prevents the server from being used to make requests to arbitrary addresses. A subscription is a personal device preference, like notification preferences: it does not publish an event or write an audit entry.

### Revocation

- **When the session ends**, its subscriptions are deleted together with the revocation: sign-out (`revoke`), sign-out from all sessions and password reset (`revoke_all_for_user`), ending one session or "all other sessions" on the security screen and password change (`MySqlAccountSessionRepository`), and member deactivation (`revoke_sessions` in `mysql_members`). Helpers in `kernel/infrastructure/notifications/push/revocation.rs`. Deleted sessions take their subscriptions with them via the FK (`ON DELETE CASCADE`).
- **On delivery**, only subscriptions whose session is active (not revoked, not expired, same user) count; the others are deleted.
- **Push service responded 404/410**: the subscription is deleted.
- **In the browser**, `registerSignOutCleanup` calls `PushSubscription.unsubscribe()` on sign-out.
- When the app is opened with permission granted and an existing subscription, the `PwaShell` resends the subscription (`syncPushSubscription`), re-linking it to the current session after a new sign-in.

### `push` channel and delivery

- `NotificationChannel::Push` (`kernel/domain/notifications/kind.rs`) applies to every kind with an in-app notice (not to `daily_digest` and `automation_email`, which are email only). On by default for `task_assigned`, `task_due`, `deal_assigned`, `note_mention`, `reminder` and `approval_*`. The preference is per user and kind, like the other channels; enabling it **on this device** is on the same screen (`PushDeviceSetting`).
- `dispatch` (`kernel/application/notifications/dispatcher.rs`), in the same transaction that creates the notification, enqueues `notification.push` (`unique_key = notification.push:<id>`) when the channel is on.
- The job (`NotificationPushHandler`, `kernel/application/notifications/push/delivery.rs`) is idempotent: it claims the notification with `notifications.pushed_at` before sending; if no delivery succeeded and some failed with a transient error, it releases the claim and fails so the worker retries.
- Delivery (`WebPushSender`): JSON payload `{ title, body, url, tag }` (title up to 120 and body up to 180 characters, `url` = the notification's relative link or `/`, `tag` = notification id), encrypted with `aes128gcm` (RFC 8291) and authenticated with VAPID ES256 (RFC 8292), using `ring` (ECDH/ECDSA P-256, HKDF-SHA256, AES-128-GCM) and `reqwest` without redirects, `TTL` of 24 h.
- In the service worker: `push` → `showNotification`; `notificationclick` → focuses an Aphel window and navigates to `url` (`OPEN_URL` message handled by the `PwaShell` with React Router) or opens a new window; `pushsubscriptionchange` → resubscribes and sends it to the backend.

### iOS

Web Push on iOS/iPadOS (16.4+) only works with Aphel **installed on the Home Screen** and opened from there. Otherwise the preferences screen shows the installation tip instead of the enable button. Permission must be requested from a user tap (the "Turn on for this device" button).

## Offline use

Aphel works with a poor connection in two areas: reading data already seen (last opened records and today's agenda) and creating tasks and notes, which go into a local queue and are sent on reconnect. All the code lives in `web/src/pwa/offline/`.

### Local storage

IndexedDB database `aphel-offline` (version 1), with no external dependency (`db.ts`):

| Store | Key | Content |
| --- | --- | --- |
| `entries` | `id` = `<userId>\|<cacheKey>` | API responses stored for offline reading (`owner`, `group`, `cacheKey`, `value`, `savedAt`). Indexes `owner` and `owner_group`. |
| `outbox` | `id` (UUID) | Creation queue (`OutboxItem`). Index `created_at`. |
| `meta` | `key` | `user`: id of the last user who used the database in this browser. |

Isolation:

- Every entry carries the owner (user id) in its key; a query only sees what was written by the same user.
- When the `OfflineShell` mounts, `claimOfflineStorage(userId)` compares it with `meta.user`; if another user signs in on the same browser, the whole database is deleted before any use.
- **Signing out deletes the local cache**: `registerSignOutCleanup(clearOfflineData)` clears the three stores and removes the database (`indexedDB.deleteDatabase`); other tabs close their connection on `versionchange`.
- Error responses are never stored. A 403/404 on read deletes the corresponding entry.

### Offline reading

`useOfflineQuery({ cacheKey, queryKey, queryFn, enabled, staleTime? })` (`cache.ts`) returns `{ query, fromCache }`:

- With network, it is a regular `useQuery` with the same `queryKey` (it shares the TanStack Query cache with the other screens) and stores the result in IndexedDB.
- On network failure (`ApiError` with `status === 0` / `network_error`) or with `navigator.onLine === false`, it returns the last stored value and `fromCache: true`. With no stored value, the network error goes through normally.
- Uses `networkMode: 'always'` (TanStack would pause the query without network) and `refetchOnReconnect: 'always'`.

Keys in use:

| `cacheKey` | Where | Limit |
| --- | --- | --- |
| `record:<type>:<id>` | `RecordDetailPage` via `useOfflineRecord` (`records.ts`) | Last 50 records opened per user (the oldest are evicted) |
| `schema:<type>` | `RecordDetailPage` via `useOfflineFieldSchema` | One per type |
| `agenda:today` | Today screen (`web/src/features/mobile`) | One |

Keys starting with `record:` count toward the recent records limit; the others do not.

### Creation queue

`submitWithOutbox({ kind: 'activity' | 'note', path, body, label })` (`outbox.ts`):

1. Creates the item with an idempotency key generated **only once** and writes it to the queue before sending (with `nextAttemptAt` postponed, so no other flush picks up the item during the direct send).
2. Without network, returns `{ status: 'queued', id }`.
3. With network, sends a `POST` with `Idempotency-Key`. Success removes the item and returns `{ status: 'sent', data }`; a network failure keeps the item in the queue and returns `queued`; other errors remove the item and are thrown for the screen to handle as in a regular submission.

Flushing the queue (`flushOutbox`) runs at app start, on the `online` event, when the window gains focus or the tab becomes visible again, and every 30 s. It:

- Uses a cross-tab lock (`navigator.locks.request(..., { ifAvailable: true })`; without Web Locks, a lease with a deadline in `localStorage`), plus an in-tab lock.
- Sends, in creation order, only the items of the session's user, re-reading each item before sending and always reusing the same key.
- Result per item:
  - 2xx (including replay with `Idempotent-Replayed: true`): removes it, shows a toast and invalidates the `tasks`, `notes` and `records` queries (detail, timeline and active lists).
  - Network: keeps it pending with exponential backoff (5 s up to 5 min) and stops sending the rest.
  - 5xx, 429, 408 and 409 `idempotency_key_in_progress`: keeps it pending with exponential backoff.
  - 401: keeps it pending and stops; resumes sending after the next sign-in.
  - Other 4xx (422 validation or `idempotency_key_reused`, 403, 404 for the linked record, 409): becomes a **conflict**.
- Items whose first attempt may have reached the server and that went more than 23 h without confirmation become an `offline_replay_window_expired` conflict, since the server only keeps the key for 24 h.

Conflicts show up in the `OfflineShell`: a floating indicator shows "Offline", the number of pending items and conflicts, and opens a dialog (`OutboxDialog`) with the list. Actions: **Try again** (goes back to the queue with a **new** key, because the server stored the 4xx response for the previous key), **Copy text** and **Discard**. The dialog also has **Send now**, which skips the backoff.

The queue state is shared across tabs through `BroadcastChannel('aphel-offline-outbox')`.

### What the server guarantees

The `back/kernel/src/api/idempotency.rs` middleware (use cases in `kernel/application/idempotency`) applies to every `POST` under `/api/v1/` (except `auth`, `installer` and public hooks), including with cookie sessions:

- The key is scoped by the credential (hash of the session cookie or of the token) and bound to the `method + path + body` fingerprint.
- Same key and same request within 24 h: returns the stored response (same status and body) with `Idempotent-Replayed: true`. 5xx and 429 responses are not stored (the key is released).
- Same key with a different request: 422 `idempotency_key_reused`. Request still in progress: 409 `idempotency_key_in_progress` (considered stuck after 10 min).
- Cleanup of expired keys is the `idempotency.purge` job.

To keep the fingerprint the same, the body is normalized (`JSON.parse(JSON.stringify(body))`) when the item is created and always sent identically.

### Known limits

- The key is per session: if the session changes (new sign-in) between an unanswered attempt and the resend, the server does not recognize the previous key and may create the item again. The risk window is only that of a send whose response was lost.
- Only the detail payload and the field schema are stored; detail tabs (timeline, attachments, emails) still require a connection.
- Queued items from another user wait until that user signs in again on the same browser.

## Field screens

Code in `web/src/features/mobile`, mounted by the `MobileShell` at the end of the `AppLayout` and visible only below `md`.

- **Bottom navigation** (`MobileNav`, fixed, with `env(safe-area-inset-bottom)`): Today (`/today`), Search (`/search`), Add ("Quick add" sheet), Inbox (`/inbox`, the tasks inbox — email has no global inbox, only a per-record tab) and More (opens the `MobileSidebar`). The `main` element gets bottom padding on mobile so it does not sit under the bar. With no home screen set in preferences, `/` goes to Today on mobile (`HomeRedirect`).
- **Today** (`TodayPage`): the user's agenda for the day (`GET /calendar` with the user's `owner_id`, from the start to the end of the day in the organization's time zone) and overdue/today's tasks (`GET /tasks/inbox?owner=me`), read through `useOfflineQuery` (`agenda:today`, `tasks:today`) with a notice when they come from the local cache.
- **Quick add** (`QuickAddSheet`, bottom sheet): contact (name, phone, email), activity, note and completed call (`POST /telephony` with outcome, direction and duration), with an optional link to a contact/company/deal through global search (`RecordPicker`; required for notes). Activity, note and check-in go through `submitWithOutbox` (`queued` shows "It will be sent when you reconnect"); contact and call require a connection.
- **Dictation**: description, note and notes fields use the Web Speech API (`SpeechRecognition`/`webkitSpeechRecognition`, `pt-BR`, partial results on screen); without support, the button disappears and a notice remains.
- **Touch**: boards (deals and custom objects) use `useBoardSensors` — `MouseSensor` (6 px distance) + `TouchSensor` with long press (250 ms, 5 px tolerance), so as not to steal scrolling — and columns with horizontal scrolling and `scroll-snap` on mobile. The record detail header, tabs and spacing were adjusted for narrow screens.

### Visit check-in

- Floating "Check-in" button on the contact, company and deal detail (mobile) and an option in "Quick add": creates a **completed** activity of type `visit` (or `meeting`, or the first active type) linked to the record.
- Location is optional: `navigator.geolocation.getCurrentPosition` with `enableHighAccuracy`, a 15 s timeout and a 30 s cache, requested when the form opens; if denied/unavailable, the check-in is saved without it (the user can try again or discard).
- API: `location: { latitude, longitude, accuracy_m, captured_at } | null` on `POST`/`PATCH /tasks` (latitude −90..90, longitude −180..180, 6 decimal places, accuracy 0..100000 m, `captured_at` defaults to now and is never in the future; `null` on `PATCH` removes it). The response includes `map_url` (OpenStreetMap). Columns `tasks.location_*` (migration `2730`), in the activity's event/audit and in the timeline metadata; "View on map" appears on the activity row, on the timeline and on the Today screen.
- Activity system fields: `location_latitude`, `location_longitude` (number, personal data), `location_accuracy_m` (number) and `location_captured_at` (date-time), filterable and sortable by the query engine. There is no geolocation field type: the four fields are written only through the `location` object (writing it counts as writing all four for the field policy) and hiding `location_latitude`/`location_longitude` removes `location` from the response.

## How to test manually

1. `npm run build` in `web/`, `cargo run -- serve --with-worker` in `back/` with `APP_ENCRYPTION_KEY` set and access over `https` or `http://localhost` (the service worker requires a secure context).
2. In Preferences → Notifications, click "Turn on for this device" and accept the permission.
3. As another user, mention the first one with `@` in a note: a push "… mentioned you in a note" arrives; clicking it opens the record.
