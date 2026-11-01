import { useSearchParams } from 'react-router'

export const RECORD_TAB_PARAM = 'tab'

export function useActiveTab(keys: string[], defaultKey?: string, param = RECORD_TAB_PARAM) {
  const [searchParams, setSearchParams] = useSearchParams()
  const fallback = defaultKey && keys.includes(defaultKey) ? defaultKey : keys[0]
  const requested = searchParams.get(param)
  const active = requested && keys.includes(requested) ? requested : fallback

  const setActive = (key: string) => {
    setSearchParams(
      (current) => {
        const next = new URLSearchParams(current)
        if (key === fallback) next.delete(param)
        else next.set(param, key)
        return next
      },
      { replace: false },
    )
  }

  return [active, setActive] as const
}
