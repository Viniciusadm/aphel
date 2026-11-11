const BASE_META_NAME = 'aphel-base'
const PUBLIC_BASE_META_NAME = 'aphel-public-base'
const SURFACE_META_NAME = 'aphel-surface'

function readMeta(name: string): string {
  if (typeof document === 'undefined') return ''
  return document.querySelector<HTMLMetaElement>(`meta[name="${name}"]`)?.content ?? ''
}

const base = readMeta(BASE_META_NAME).replace(/\/+$/, '')
const publicBase = readMeta(PUBLIC_BASE_META_NAME).replace(/\/+$/, '')
const siteSurface = readMeta(SURFACE_META_NAME) === 'site'

export function appBase(): string {
  return base
}

export function appPath(path: string): string {
  return `${base}${path.startsWith('/') ? path : `/${path}`}`
}

export function appUrl(path: string): string {
  return `${window.location.origin}${appPath(path)}`
}

export function publicUrl(path: string): string {
  const normalized = path.startsWith('/') ? path : `/${path}`
  return publicBase ? `${publicBase}${normalized}` : appUrl(normalized)
}

export function onSiteSurface(): boolean {
  return siteSurface
}
