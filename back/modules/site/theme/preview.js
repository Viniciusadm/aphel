const READY = 'aphel:preview-ready'
const HTML = 'aphel:preview-html'
const BANNER = '[data-aphel-preview-banner]'

let lastAnchor = null

function syncHead(next) {
  const current = [...document.head.querySelectorAll('style')]
  const incoming = [...next.head.querySelectorAll('style')]
  current.forEach((style, index) => {
    const replacement = incoming[index]
    if (!replacement) style.remove()
    else if (style.textContent !== replacement.textContent) style.textContent = replacement.textContent
  })
  for (const style of incoming.slice(current.length)) document.head.append(document.importNode(style, true))
  if (next.title) document.title = next.title
}

const ELEMENT = 1
const MOUNTED = 'data-island-mounted'

function preserved(node) {
  return node.nodeType === ELEMENT && (node.matches(BANNER) || (node.localName === 'script' && (node.getAttribute('src') || '').endsWith('/site/preview.js')))
}

function runnable(old) {
  const script = document.createElement('script')
  for (const { name, value } of old.attributes) script.setAttribute(name, value)
  script.textContent = old.textContent
  return script
}

function fresh(node) {
  const adopted = document.adoptNode(node)
  if (adopted.nodeType !== ELEMENT) return adopted
  if (adopted.localName === 'script') return runnable(adopted)
  for (const old of adopted.querySelectorAll('script')) old.replaceWith(runnable(old))
  return adopted
}

function sameAttributes(current, next, ignored) {
  const names = (element) => [...element.attributes].map((attribute) => attribute.name).filter((name) => name !== ignored)
  const currentNames = names(current)
  return currentNames.length === names(next).length && currentNames.every((name) => current.getAttribute(name) === next.getAttribute(name))
}

function syncAttributes(current, next) {
  for (const { name } of [...current.attributes]) if (!next.hasAttribute(name)) current.removeAttribute(name)
  for (const { name, value } of next.attributes) if (current.getAttribute(name) !== value) current.setAttribute(name, value)
}

function morph(current, next) {
  if (current.nodeType !== ELEMENT) {
    if (current.nodeValue !== next.nodeValue) current.nodeValue = next.nodeValue
    return current
  }
  if (current.localName === 'script') return current.outerHTML === next.outerHTML ? current : fresh(next)
  if (current.localName === 'iframe') {
    if (current.getAttribute('src') !== next.getAttribute('src')) return fresh(next)
    syncAttributes(current, next)
    return current
  }
  if (current.hasAttribute(MOUNTED)) return sameAttributes(current, next, MOUNTED) ? current : fresh(next)
  syncAttributes(current, next)
  morphChildren(current, next)
  return current
}

function compatible(current, next) {
  return current.nodeType === next.nodeType && current.nodeName === next.nodeName && (current.nodeType !== ELEMENT || current.id === next.id)
}

function morphChildren(parent, source) {
  const current = [...parent.childNodes].filter((node) => !preserved(node))
  const byId = new Map()
  for (const node of current) if (node.nodeType === ELEMENT && node.id) byId.set(node.id, node)
  const used = new Set()
  const result = []
  let cursor = 0
  for (const next of [...source.childNodes]) {
    let match = null
    if (next.nodeType === ELEMENT && next.id) {
      const candidate = byId.get(next.id)
      if (candidate && !used.has(candidate) && candidate.nodeName === next.nodeName) match = candidate
    } else {
      while (cursor < current.length && (used.has(current[cursor]) || (current[cursor].nodeType === ELEMENT && current[cursor].id))) cursor += 1
      const candidate = current[cursor]
      if (candidate && compatible(candidate, next)) {
        match = candidate
        cursor += 1
      }
    }
    if (match) used.add(match)
    result.push(match ? morph(match, next) : fresh(next))
  }
  const kept = new Set(result)
  for (const node of current) if (!kept.has(node)) node.remove()
  let expected = parent.firstChild
  for (const node of result) {
    if (node === expected) expected = expected.nextSibling
    else parent.insertBefore(node, expected)
  }
}

function apply(html, anchor) {
  const next = new DOMParser().parseFromString(html, 'text/html')
  const scroll = window.scrollY
  syncHead(next)
  if (document.body.className !== next.body.className) document.body.className = next.body.className
  morphChildren(document.body, next.body)
  document.dispatchEvent(new CustomEvent('aphel:islands'))
  const target = anchor && anchor !== lastAnchor ? document.getElementById(anchor) : null
  if (target) target.scrollIntoView({ block: 'start' })
  else window.scrollTo(0, scroll)
  lastAnchor = anchor || null
}

window.addEventListener('message', (event) => {
  if (event.source !== window.parent || event.origin !== window.location.origin) return
  const data = event.data
  if (!data || data.type !== HTML || typeof data.html !== 'string') return
  apply(data.html, typeof data.anchor === 'string' ? data.anchor : null)
})

if (window.parent !== window) window.parent.postMessage({ type: READY }, window.location.origin)
