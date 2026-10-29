const LEASE_STORAGE_PREFIX = 'aphel.offline.lock.'
const LEASE_DURATION_MS = 60_000
const LEASE_SETTLE_MS = 50

const tabId = typeof crypto !== 'undefined' && 'randomUUID' in crypto ? crypto.randomUUID() : String(Math.random())
const heldInTab = new Set<string>()

interface Lease {
  holder: string
  expiresAt: number
}

function readLease(key: string): Lease | null {
  try {
    const raw = localStorage.getItem(key)
    return raw ? (JSON.parse(raw) as Lease) : null
  } catch {
    return null
  }
}

function writeLease(key: string, lease: Lease | null): void {
  try {
    if (lease) localStorage.setItem(key, JSON.stringify(lease))
    else localStorage.removeItem(key)
  } catch {
    return
  }
}

function wait(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds))
}

async function withLease<T>(name: string, run: () => Promise<T>): Promise<T | null> {
  const key = `${LEASE_STORAGE_PREFIX}${name}`
  const current = readLease(key)
  if (current && current.holder !== tabId && current.expiresAt > Date.now()) return null
  writeLease(key, { holder: tabId, expiresAt: Date.now() + LEASE_DURATION_MS })
  await wait(LEASE_SETTLE_MS)
  if (readLease(key)?.holder !== tabId) return null
  try {
    return await run()
  } finally {
    if (readLease(key)?.holder === tabId) writeLease(key, null)
  }
}

export async function withExclusiveLock<T>(name: string, run: () => Promise<T>): Promise<T | null> {
  if (heldInTab.has(name)) return null
  heldInTab.add(name)
  try {
    if (typeof navigator !== 'undefined' && navigator.locks) {
      return await navigator.locks.request(name, { ifAvailable: true }, async (lock) => (lock ? run() : null))
    }
    return await withLease(name, run)
  } finally {
    heldInTab.delete(name)
  }
}
