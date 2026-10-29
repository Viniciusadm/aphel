import type { QueryClient } from '@tanstack/react-query'
import { SESSION_QUERY_KEY } from '@/app/session/session-query'
import { registerSignOutCleanup } from '@/pwa/sign-out'
import { ENTRIES_STORE, META_STORE, clearOfflineDatabase, isIndexedDbAvailable, requestResult, withStores } from './db'
import { resetOutboxSnapshot } from './outbox'
import { getCurrentOwner, setCurrentOwner } from './owner'

const OWNER_META_KEY = 'user'

interface MetaEntry {
  key: string
  value: string
}

export async function clearOfflineData(): Promise<void> {
  setCurrentOwner(null)
  resetOutboxSnapshot()
  await clearOfflineDatabase()
}

export async function clearOfflineCache(): Promise<void> {
  if (!isIndexedDbAvailable()) return
  await withStores(ENTRIES_STORE, 'readwrite', (transaction) => {
    transaction.objectStore(ENTRIES_STORE).clear()
  })
}

function isSessionKey(queryKey: readonly unknown[]): boolean {
  return SESSION_QUERY_KEY.every((part, index) => queryKey[index] === part)
}

export function watchSessionExpiry(queryClient: QueryClient): () => void {
  return queryClient.getQueryCache().subscribe((event) => {
    if (event.type !== 'updated' || event.action.type !== 'success') return
    if (!isSessionKey(event.query.queryKey) || event.query.state.data !== null) return
    if (!getCurrentOwner()) return
    setCurrentOwner(null)
    resetOutboxSnapshot()
    void clearOfflineCache().catch(() => undefined)
  })
}

export async function claimOfflineStorage(userId: string): Promise<void> {
  if (!isIndexedDbAvailable()) return
  const stored = await withStores(META_STORE, 'readonly', (transaction) =>
    requestResult<MetaEntry | undefined>(transaction.objectStore(META_STORE).get(OWNER_META_KEY)),
  )
  if (stored?.value === userId) return
  if (stored) await clearOfflineDatabase()
  await withStores(META_STORE, 'readwrite', (transaction) => {
    transaction.objectStore(META_STORE).put({ key: OWNER_META_KEY, value: userId } satisfies MetaEntry)
  })
}

let installed = false

export function installOfflineSignOutCleanup(): void {
  if (installed) return
  installed = true
  registerSignOutCleanup(clearOfflineData)
}
