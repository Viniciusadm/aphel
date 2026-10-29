export const DEFAULT_REDIRECT = '/'
export const REDIRECT_PARAM = 'redirect'

export function safeRedirectPath(value: string | null | undefined): string {
  if (!value || !value.startsWith('/')) return DEFAULT_REDIRECT
  if (value.startsWith('//') || value.startsWith('/\\')) return DEFAULT_REDIRECT
  return value
}

export function loginPathFor(currentPath: string): string {
  const target = safeRedirectPath(currentPath)
  if (target === DEFAULT_REDIRECT) return '/login'
  return `/login?${new URLSearchParams({ [REDIRECT_PARAM]: target }).toString()}`
}

export function withRedirect(path: string, redirect: string | null): string {
  if (!redirect) return path
  return `${path}?${new URLSearchParams({ [REDIRECT_PARAM]: safeRedirectPath(redirect) }).toString()}`
}
