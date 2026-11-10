const BASE_META_NAME = 'aphel-base'

function readAppBase(): string {
  if (typeof document === 'undefined') return ''
  const content = document.querySelector<HTMLMetaElement>(`meta[name="${BASE_META_NAME}"]`)?.content ?? ''
  return content.replace(/\/+$/, '')
}

const base = readAppBase()

export function appBase(): string {
  return base
}

export function appPath(path: string): string {
  return `${base}${path.startsWith('/') ? path : `/${path}`}`
}

export function appUrl(path: string): string {
  return `${window.location.origin}${appPath(path)}`
}
