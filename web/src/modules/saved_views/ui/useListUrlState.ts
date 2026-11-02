import { useCallback, useMemo } from 'react'
import { useSearchParams } from 'react-router'
import { buildListSearch, parseListSearch, type ListUrlState } from '@/modules/saved_views/ui/urlState'

export type ListUrlStateUpdate = ListUrlState | ((current: ListUrlState) => ListUrlState)

export interface SetListUrlStateOptions {
  replace?: boolean
}

export function useListUrlState() {
  const [searchParams, setSearchParams] = useSearchParams()
  const serialized = searchParams.toString()
  const state = useMemo(() => parseListSearch(new URLSearchParams(serialized)), [serialized])

  const setState = useCallback(
    (update: ListUrlStateUpdate, options: SetListUrlStateOptions = {}) => {
      setSearchParams(
        (current) => {
          const next = typeof update === 'function' ? update(parseListSearch(current)) : update
          return buildListSearch(next, current)
        },
        { replace: options.replace },
      )
    },
    [setSearchParams],
  )

  return [state, setState] as const
}
