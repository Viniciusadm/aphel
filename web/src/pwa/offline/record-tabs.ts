import type { Query, QueryClient, QueryKey } from '@tanstack/react-query'
import { TAB_CACHE_PREFIX, readCachedValue, writeCachedValue } from './cache'
import { isNetworkFailure, isOnline } from './network'
import { getCurrentOwner } from './owner'

type KeyMatcher = (queryKey: QueryKey) => boolean

const RECORD_TAB_QUERIES: KeyMatcher[] = [
  (key) => key[0] === 'records' && key[2] === 'detail' && key[4] === 'timeline',
  (key) => key[0] === 'files' && key[1] === 'attachments' && key.length === 4,
  (key) => key[0] === 'notes' && key.length === 3,
  (key) => key[0] === 'tasks' && key[1] === 'record' && key.length === 4,
]

function tabCacheKey(queryKey: QueryKey): string | null {
  if (!RECORD_TAB_QUERIES.some((matches) => matches(queryKey))) return null
  return `${TAB_CACHE_PREFIX}${JSON.stringify(queryKey)}`
}

async function hydrate(queryClient: QueryClient, query: Query, cacheKey: string): Promise<void> {
  const owner = getCurrentOwner()
  if (!owner || query.state.data !== undefined) return
  const cached = await readCachedValue<unknown>(owner, cacheKey).catch(() => null)
  if (!cached || query.state.data !== undefined) return
  queryClient.setQueryData(query.queryKey, cached.value, { updatedAt: cached.savedAt })
}

export function persistRecordTabs(queryClient: QueryClient): () => void {
  const cache = queryClient.getQueryCache()
  if (!isOnline()) {
    for (const query of cache.getAll()) {
      const cacheKey = tabCacheKey(query.queryKey)
      if (cacheKey) void hydrate(queryClient, query, cacheKey)
    }
  }
  return cache.subscribe((event) => {
    const cacheKey = tabCacheKey(event.query.queryKey)
    if (!cacheKey) return
    if (event.type === 'added') {
      if (!isOnline()) void hydrate(queryClient, event.query, cacheKey)
      return
    }
    if (event.type !== 'updated') return
    const { action } = event
    if (action.type === 'error' && isNetworkFailure(action.error)) {
      void hydrate(queryClient, event.query, cacheKey)
      return
    }
    if (action.type !== 'success' || action.manual) return
    const owner = getCurrentOwner()
    if (!owner) return
    void writeCachedValue(owner, cacheKey, event.query.state.data).catch(() => undefined)
  })
}
