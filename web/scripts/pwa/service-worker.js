const VERSION = '__APHEL_SW_VERSION__'
const PRECACHE_URLS = ['__APHEL_SW_PRECACHE__']
const CACHE_PREFIX = 'aphel-shell-'
const CACHE_NAME = `${CACHE_PREFIX}${VERSION}`
const SHELL_URL = new URL(self.registration.scope).pathname
const RESERVED_PREFIX = '/_aphel/'
const ASSETS_PREFIX = `${RESERVED_PREFIX}assets/`
const SHARED_PREFIX = `${RESERVED_PREFIX}shared/`
const ICON_URL = `${RESERVED_PREFIX}icon-192.png`
const PLUGINS_CACHE = 'aphel-plugins'
const PLUGINS_LIST_URL = '/api/v1/public/wasm-runtime'
const PLUGIN_ASSETS_PREFIX = `${PLUGINS_LIST_URL}/`
const BYPASS_PREFIXES = ['/api/', RESERVED_PREFIX, `${SHELL_URL}p/`]
const CSRF_HEADER = 'X-Requested-With'
const CSRF_VALUE = 'aphel'
const SUBSCRIPTIONS_URL = '/api/v1/notifications/push/subscriptions'

self.addEventListener('install', (event) => {
  event.waitUntil(caches.open(CACHE_NAME).then((cache) => cache.addAll([SHELL_URL, ...PRECACHE_URLS])))
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

function pluginAssetKey(url) {
  return url.pathname.slice(PLUGIN_ASSETS_PREFIX.length).split('/')[0]
}

async function prunePluginAssets(cache, list) {
  const body = await list.json()
  const versions = new Map()
  for (const plugin of body.data) {
    versions.set(plugin.key, new URL(plugin.entry, self.location.origin).searchParams.get('v'))
  }
  const requests = await cache.keys()
  await Promise.all(
    requests.map((request) => {
      const url = new URL(request.url)
      if (url.pathname === PLUGINS_LIST_URL) return false
      const key = pluginAssetKey(url)
      const version = url.searchParams.get('v')
      const current = versions.has(key) && (version === null || version === versions.get(key))
      return current ? false : cache.delete(request)
    }),
  )
}

async function networkFirstPluginList(request) {
  const cache = await caches.open(PLUGINS_CACHE)
  try {
    const response = await fetch(request)
    if (response.ok) {
      await cache.put(PLUGINS_LIST_URL, response.clone())
      await prunePluginAssets(cache, response.clone()).catch(() => undefined)
    }
    return response
  } catch (error) {
    const cached = await cache.match(PLUGINS_LIST_URL)
    if (cached) return cached
    throw error
  }
}

async function pluginAsset(request, url) {
  const cache = await caches.open(PLUGINS_CACHE)
  const cached = await cache.match(request)
  if (cached && url.searchParams.has('v')) return cached
  try {
    const response = await fetch(request)
    if (response.ok) await cache.put(request, response.clone())
    return response
  } catch (error) {
    if (cached) return cached
    throw error
  }
}

self.addEventListener('fetch', (event) => {
  const { request } = event
  if (request.method !== 'GET') return
  const url = new URL(request.url)
  if (url.origin !== self.location.origin) return
  if (url.pathname === PLUGINS_LIST_URL) {
    event.respondWith(networkFirstPluginList(request))
    return
  }
  if (url.pathname.startsWith(PLUGIN_ASSETS_PREFIX)) {
    event.respondWith(pluginAsset(request, url))
    return
  }
  if (url.pathname.startsWith(ASSETS_PREFIX) || url.pathname.startsWith(SHARED_PREFIX)) {
    event.respondWith(cacheFirstAsset(request))
    return
  }
  if (isBypassed(url)) return
  if (request.mode === 'navigate') {
    event.respondWith(networkFirstShell(request))
  }
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
      icon: ICON_URL,
      data: { url: appRelativePath(payload.url) },
    }),
  )
})

function appRelativePath(value) {
  return typeof value === 'string' && value.startsWith('/') && !value.startsWith('//') ? value : '/'
}

function isAppWindow(candidate) {
  const url = new URL(candidate.url)
  return url.origin === self.location.origin && url.pathname.startsWith(SHELL_URL)
}

async function openUrl(path) {
  const windows = await self.clients.matchAll({ type: 'window', includeUncontrolled: true })
  const client = windows.find(isAppWindow)
  if (client) {
    await client.focus()
    client.postMessage({ type: 'OPEN_URL', url: path })
    return
  }
  await self.clients.openWindow(new URL(path.slice(1), self.registration.scope).href)
}

self.addEventListener('notificationclick', (event) => {
  event.notification.close()
  const data = event.notification.data || {}
  event.waitUntil(openUrl(appRelativePath(data.url)))
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
