const VERSION = '__APHEL_SW_VERSION__'
const PRECACHE_URLS = ['__APHEL_SW_PRECACHE__']
const CACHE_PREFIX = 'aphel-shell-'
const CACHE_NAME = `${CACHE_PREFIX}${VERSION}`
const SHELL_URL = '/'
const ASSETS_PREFIX = '/assets/'
const BYPASS_PREFIXES = ['/api/', '/health']
const CSRF_HEADER = 'X-Requested-With'
const CSRF_VALUE = 'aphel'
const SUBSCRIPTIONS_URL = '/api/v1/notifications/push/subscriptions'

self.addEventListener('install', (event) => {
  event.waitUntil(caches.open(CACHE_NAME).then((cache) => cache.addAll(PRECACHE_URLS)))
})

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((names) =>
        Promise.all(
          names
            .filter((name) => name.startsWith(CACHE_PREFIX) && name !== CACHE_NAME)
            .map((name) => caches.delete(name)),
        ),
      )
      .then(() => self.clients.claim()),
  )
})

self.addEventListener('message', (event) => {
  if (event.data && event.data.type === 'SKIP_WAITING') self.skipWaiting()
})

function isBypassed(url) {
  return url.pathname === '/api' || BYPASS_PREFIXES.some((prefix) => url.pathname.startsWith(prefix))
}

function isHtml(response) {
  return (response.headers.get('content-type') || '').includes('text/html')
}

async function networkFirstShell(request) {
  const cache = await caches.open(CACHE_NAME)
  try {
    const response = await fetch(request)
    if (response.ok && isHtml(response)) await cache.put(SHELL_URL, response.clone())
    return response
  } catch (error) {
    const cached = await cache.match(SHELL_URL)
    if (cached) return cached
    throw error
  }
}

async function cacheFirstAsset(request) {
  const cache = await caches.open(CACHE_NAME)
  const cached = await cache.match(request)
  if (cached) return cached
  const response = await fetch(request)
  if (response.ok) await cache.put(request, response.clone())
  return response
}

self.addEventListener('fetch', (event) => {
  const { request } = event
  if (request.method !== 'GET') return
  const url = new URL(request.url)
  if (url.origin !== self.location.origin || isBypassed(url)) return
  if (request.mode === 'navigate') {
    event.respondWith(networkFirstShell(request))
    return
  }
  if (url.pathname.startsWith(ASSETS_PREFIX)) event.respondWith(cacheFirstAsset(request))
})

function pushPayload(event) {
  if (!event.data) return {}
  try {
    return event.data.json()
  } catch {
    return { body: event.data.text() }
  }
}

self.addEventListener('push', (event) => {
  const payload = pushPayload(event)
  const title = typeof payload.title === 'string' && payload.title ? payload.title : 'Aphel'
  event.waitUntil(
    self.registration.showNotification(title, {
      body: typeof payload.body === 'string' ? payload.body : undefined,
      tag: typeof payload.tag === 'string' ? payload.tag : undefined,
      icon: '/icon-192.png',
      data: { url: typeof payload.url === 'string' ? payload.url : SHELL_URL },
    }),
  )
})

function sameOriginUrl(value) {
  try {
    const url = new URL(value || SHELL_URL, self.location.origin)
    return url.origin === self.location.origin ? url : new URL(SHELL_URL, self.location.origin)
  } catch {
    return new URL(SHELL_URL, self.location.origin)
  }
}

async function openUrl(url) {
  const windows = await self.clients.matchAll({ type: 'window', includeUncontrolled: true })
  const client = windows.find((candidate) => new URL(candidate.url).origin === self.location.origin)
  if (client) {
    await client.focus()
    client.postMessage({ type: 'OPEN_URL', url: `${url.pathname}${url.search}${url.hash}` })
    return
  }
  await self.clients.openWindow(url.href)
}

self.addEventListener('notificationclick', (event) => {
  event.notification.close()
  const data = event.notification.data || {}
  event.waitUntil(openUrl(sameOriginUrl(data.url)))
})

function keyToBase64Url(buffer) {
  if (!buffer) return ''
  const bytes = new Uint8Array(buffer)
  let binary = ''
  for (const byte of bytes) binary += String.fromCharCode(byte)
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

async function resubscribe(event) {
  const options = event.oldSubscription ? event.oldSubscription.options : null
  if (!options || !options.applicationServerKey) return
  const subscription = await self.registration.pushManager.subscribe({
    userVisibleOnly: true,
    applicationServerKey: options.applicationServerKey,
  })
  await fetch(SUBSCRIPTIONS_URL, {
    method: 'POST',
    credentials: 'same-origin',
    headers: { 'Content-Type': 'application/json', [CSRF_HEADER]: CSRF_VALUE },
    body: JSON.stringify({
      endpoint: subscription.endpoint,
      keys: {
        p256dh: keyToBase64Url(subscription.getKey('p256dh')),
        auth: keyToBase64Url(subscription.getKey('auth')),
      },
    }),
  })
}

self.addEventListener('pushsubscriptionchange', (event) => {
  event.waitUntil(resubscribe(event).catch(() => undefined))
})
