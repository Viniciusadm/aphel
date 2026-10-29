import { TODAY_PATH } from '../routes'

export const MOBILE_HOME_QUERY = '(max-width: 767px)'

export function mobileHomePath(): string | null {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return null
  return window.matchMedia(MOBILE_HOME_QUERY).matches ? TODAY_PATH : null
}
