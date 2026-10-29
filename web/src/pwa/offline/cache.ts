import { useQuery, type QueryKey, type UseQueryResult } from '@tanstack/react-query'
import { useSyncExternalStore } from 'react'
import { isApiError } from '@/api'
import { ENTRIES_STORE, isIndexedDbAvailable, requestResult, withStores } from './db'
import { isNetworkFailure, isOnline } from './network'
import { ownerKey, useOfflineOwner, type OfflineOwner } from './owner'

export interface OfflineQueryOptions<T> {
  cacheKey: string
  queryKey: QueryKey
  queryFn: () => Promise<T>
  enabled?: boolean
  staleTime?: number
}

export interface OfflineQueryResult<T> {
  query: UseQueryResult<T>
  fromCache: boolean
}

export const RECENT_RECORDS_LIMIT = 50
export const RECENT_TABS_LIMIT = 200
const RECORD_CACHE_PREFIX = 'record:'
export const TAB_CACHE_PREFIX = 'tab:'

type EntryGroup = 'record' | 'tab' | 'other'

const GROUP_LIMITS: Partial<Record<EntryGroup, number>> = {
  record: RECENT_RECORDS_LIMIT,
  tab: RECENT_TABS_LIMIT,
}

interface CacheEntry {
  id: string
  owner: string
  group: EntryGroup
  cacheKey: string
  value: unknown
  savedAt: number
}

export interface CachedValue<T> {
  value: T
  savedAt: number
}

function entryId(owner: OfflineOwner, cacheKey: string): string {
  return `${ownerKey(owner)}|${cacheKey}`
}

function groupOf(cacheKey: string): EntryGroup {
  if (cacheKey.startsWith(RECORD_CACHE_PREFIX)) return 'record'
  if (cacheKey.startsWith(TAB_CACHE_PREFIX)) return 'tab'
  return 'other'
}

export async function readCachedValue<T>(owner: OfflineOwner, cacheKey: string): Promise<CachedValue<T> | null> {
  if (!isIndexedDbAvailable()) return null
  const entry = await withStores(ENTRIES_STORE, 'readonly', (transaction) =>
    requestResult<CacheEntry | undefined>(transaction.objectStore(ENTRIES_STORE).get(entryId(owner, cacheKey))),
  )
  return entry ? { value: entry.value as T, savedAt: entry.savedAt } : null
}

export async function writeCachedValue(owner: OfflineOwner, cacheKey: string, value: unknown): Promise<void> {
  if (!isIndexedDbAvailable()) return
  const group = groupOf(cacheKey)
  const entry: CacheEntry = {
    id: entryId(owner, cacheKey),
    owner: ownerKey(owner),
    group,
    cacheKey,
    value,
    savedAt: Date.now(),
  }
  await withStores(ENTRIES_STORE, 'readwrite', async (transaction) => {
    const store = transaction.objectStore(ENTRIES_STORE)
    store.put(entry)
    const limit = GROUP_LIMITS[group]
    if (limit === undefined) return
    const entries = await requestResult<CacheEntry[]>(
      store.index('owner_group').getAll(IDBKeyRange.only([entry.owner, group])),
    )
    entries
      .sort((left, right) => right.savedAt - left.savedAt)
      .slice(limit)
      .forEach((stale) => store.delete(stale.id))
  })
}

export async function deleteCachedValue(owner: OfflineOwner, cacheKey: string): Promise<void> {
  if (!isIndexedDbAvailable()) return
  await withStores(ENTRIES_STORE, 'readwrite', (transaction) => {
    transaction.objectStore(ENTRIES_STORE).delete(entryId(owner, cacheKey))
  })
}

function isRevoked(error: unknown): boolean {
  return isApiError(error) && (error.status === 403 || error.status === 404)
}

const servedFromCache = new Set<string>()
const flagListeners = new Set<() => void>()

function setServedFromCache(flag: string, value: boolean): void {
  if (servedFromCache.has(flag) === value) return
  if (value) servedFromCache.add(flag)
  else servedFromCache.delete(flag)
  for (const listener of flagListeners) listener()
}

function subscribeToFlags(listener: () => void): () => void {
  flagListeners.add(listener)
  return () => {
    flagListeners.delete(listener)
  }
}

async function cachedOrNull<T>(owner: OfflineOwner, cacheKey: string): Promise<CachedValue<T> | null> {
  return readCachedValue<T>(owner, cacheKey).catch(() => null)
}

export async function fetchWithOfflineCache<T>(
  owner: OfflineOwner | null,
  cacheKey: string,
  queryFn: () => Promise<T>,
): Promise<T> {
  if (!owner) return queryFn()
  const flag = entryId(owner, cacheKey)
  if (!isOnline()) {
    const cached = await cachedOrNull<T>(owner, cacheKey)
    if (cached) {
      setServedFromCache(flag, true)
      return cached.value
    }
  }
  try {
    const value = await queryFn()
    setServedFromCache(flag, false)
    void writeCachedValue(owner, cacheKey, value).catch(() => undefined)
    return value
  } catch (error) {
    if (isRevoked(error)) void deleteCachedValue(owner, cacheKey).catch(() => undefined)
    if (!isNetworkFailure(error)) throw error
    const cached = await cachedOrNull<T>(owner, cacheKey)
    if (!cached) throw error
    setServedFromCache(flag, true)
    return cached.value
  }
}

export function useOfflineQuery<T>(options: OfflineQueryOptions<T>): OfflineQueryResult<T> {
  const owner = useOfflineOwner()
  const flag = owner ? entryId(owner, options.cacheKey) : null
  const query = useQuery({
    queryKey: options.queryKey,
    queryFn: () => fetchWithOfflineCache(owner, options.cacheKey, options.queryFn),
    enabled: options.enabled,
    ...(options.staleTime === undefined ? {} : { staleTime: options.staleTime }),
    networkMode: 'always',
    refetchOnReconnect: 'always',
  })
  const flagged = useSyncExternalStore(
    subscribeToFlags,
    () => (flag ? servedFromCache.has(flag) : false),
    () => false,
  )
  return { query, fromCache: flagged && query.data !== undefined }
}
