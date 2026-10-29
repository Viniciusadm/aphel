import { useCallback, useSyncExternalStore } from 'react'

export const SMALL_SCREEN_QUERY = '(max-width: 639px)'

export function useMediaQuery(query: string): boolean {
  const subscribe = useCallback(
    (onChange: () => void) => {
      if (typeof window === 'undefined' || !window.matchMedia) return () => undefined
      const list = window.matchMedia(query)
      list.addEventListener('change', onChange)
      return () => list.removeEventListener('change', onChange)
    },
    [query],
  )
  const snapshot = useCallback(
    () => (typeof window !== 'undefined' && window.matchMedia ? window.matchMedia(query).matches : false),
    [query],
  )
  return useSyncExternalStore(subscribe, snapshot, () => false)
}
