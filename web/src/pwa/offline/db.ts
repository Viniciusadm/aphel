export const OFFLINE_DB_NAME = 'aphel-offline'
const OFFLINE_DB_VERSION = 1

export const ENTRIES_STORE = 'entries'
export const OUTBOX_STORE = 'outbox'
export const META_STORE = 'meta'

const ALL_STORES = [ENTRIES_STORE, OUTBOX_STORE, META_STORE] as const

export type StoreName = (typeof ALL_STORES)[number]

let connection: Promise<IDBDatabase> | null = null

export function isIndexedDbAvailable(): boolean {
  return typeof indexedDB !== 'undefined'
}

function upgrade(database: IDBDatabase): void {
  if (!database.objectStoreNames.contains(ENTRIES_STORE)) {
    const entries = database.createObjectStore(ENTRIES_STORE, { keyPath: 'id' })
    entries.createIndex('owner', 'owner')
    entries.createIndex('owner_group', ['owner', 'group'])
  }
  if (!database.objectStoreNames.contains(OUTBOX_STORE)) {
    const outbox = database.createObjectStore(OUTBOX_STORE, { keyPath: 'id' })
    outbox.createIndex('created_at', 'createdAt')
  }
  if (!database.objectStoreNames.contains(META_STORE)) {
    database.createObjectStore(META_STORE, { keyPath: 'key' })
  }
}

function openDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(OFFLINE_DB_NAME, OFFLINE_DB_VERSION)
    request.onupgradeneeded = () => upgrade(request.result)
    request.onsuccess = () => {
      const database = request.result
      database.onversionchange = () => {
        database.close()
        connection = null
      }
      database.onclose = () => {
        connection = null
      }
      resolve(database)
    }
    request.onerror = () => reject(request.error ?? new Error('indexeddb open failed'))
    request.onblocked = () => reject(new Error('indexeddb open blocked'))
  })
}

export function getDatabase(): Promise<IDBDatabase> {
  if (!isIndexedDbAvailable()) return Promise.reject(new Error('indexeddb unavailable'))
  if (!connection) {
    connection = openDatabase().catch((error: unknown) => {
      connection = null
      throw error
    })
  }
  return connection
}

export function requestResult<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result)
    request.onerror = () => reject(request.error ?? new Error('indexeddb request failed'))
  })
}

function transactionDone(transaction: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    transaction.oncomplete = () => resolve()
    transaction.onerror = () => reject(transaction.error ?? new Error('indexeddb transaction failed'))
    transaction.onabort = () => reject(transaction.error ?? new Error('indexeddb transaction aborted'))
  })
}

export async function withStores<T>(
  stores: StoreName | StoreName[],
  mode: IDBTransactionMode,
  run: (transaction: IDBTransaction) => Promise<T> | T,
): Promise<T> {
  const database = await getDatabase()
  const transaction = database.transaction(stores, mode)
  const done = transactionDone(transaction)
  done.catch(() => undefined)
  const result = await run(transaction)
  await done
  return result
}

function clearStores(): Promise<void> {
  return withStores([...ALL_STORES], 'readwrite', (transaction) => {
    for (const store of ALL_STORES) transaction.objectStore(store).clear()
  })
}

async function closeConnection(): Promise<void> {
  const current = connection
  connection = null
  if (!current) return
  const database = await current.catch(() => null)
  database?.close()
}

function deleteDatabase(): Promise<void> {
  return new Promise((resolve) => {
    const request = indexedDB.deleteDatabase(OFFLINE_DB_NAME)
    request.onsuccess = () => resolve()
    request.onerror = () => resolve()
    request.onblocked = () => resolve()
  })
}

export async function clearOfflineDatabase(): Promise<void> {
  if (!isIndexedDbAvailable()) return
  await clearStores().catch(() => undefined)
  await closeConnection()
  await deleteDatabase()
}
