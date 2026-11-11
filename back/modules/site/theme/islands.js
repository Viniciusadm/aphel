const API_HEADER = 'X-Aphel-Site'

function contextFor(owner) {
  const base = `/_aphel/site/api/${owner}`
  return {
    owner,
    async api(path, init = {}) {
      const headers = new Headers(init.headers || {})
      headers.set(API_HEADER, '1')
      let body = init.body
      if (body !== undefined && typeof body !== 'string' && !(body instanceof FormData)) {
        headers.set('Content-Type', 'application/json')
        body = JSON.stringify(body)
      }
      const response = await fetch(`${base}${path.startsWith('/') ? path : `/${path}`}`, {
        ...init,
        body,
        headers,
        credentials: 'same-origin',
      })
      const text = await response.text()
      const data = text ? JSON.parse(text) : null
      if (!response.ok) {
        const error = new Error(data?.error?.message || `HTTP ${response.status}`)
        error.status = response.status
        error.code = data?.error?.code
        throw error
      }
      return data
    },
  }
}

async function mount(element) {
  const [owner, name] = (element.dataset.island || '').split(':')
  if (!owner || !name) return
  let props = {}
  try {
    props = JSON.parse(element.dataset.props || '{}')
  } catch {
    props = {}
  }
  try {
    const module = await import(`/_aphel/site/islands/${owner}/${name}.js`)
    const start = module.default || module.mount
    if (typeof start === 'function') await start(element, props, contextFor(owner))
  } catch (error) {
    console.error(`[aphel site] island ${owner}:${name} failed`, error)
  }
}

for (const element of document.querySelectorAll('[data-island]')) void mount(element)
