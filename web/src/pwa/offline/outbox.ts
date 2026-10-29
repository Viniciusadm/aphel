import { useSyncExternalStore } from 'react'
import { api, isApiError } from '@/api'
import { OUTBOX_STORE, isIndexedDbAvailable, requestResult, withStores } from './db'
import { withExclusiveLock } from './lock'
import { isNetworkFailure, isOnline } from './network'
import { getCurrentOwner, type OfflineOwner } from './owner'

export type OutboxKind = 'activity' | 'note' | 'contact' | 'call'

export interface OutboxSubmission {
  kind: OutboxKind
  path: string
  body: unknown
  label: string
}

export type OutboxResult<T> = { status: 'sent'; data: T } | { status: 'queued'; id: string }

export const IDEMPOTENCY_KEY_HEADER = 'Idempotency-Key'

export const IDEMPOTENCY_KEY_IN_PROGRESS = 'idempotency_key_in_progress'
export const REPLAY_WINDOW_EXPIRED = 'offline_replay_window_expired'

export type OutboxStatus = 'pending' | 'conflict'

export interface OutboxError {
  status: number
  code: string
  message: string
}

export interface OutboxItem {
  id: string
  idempotencyKey: string
  kind: OutboxKind
  path: string
  body: unknown
  label: string
  userId: string
  createdAt: number
  status: OutboxStatus
  attempts: number
  nextAttemptAt: number
  firstAttemptAt: number | null
  uncertain: boolean
  error: OutboxError | null
}

export interface OutboxSnapshot {
  pending: OutboxItem[]
  conflicts: OutboxItem[]
}

export interface OutboxDelivery {
  item: OutboxItem
  data: unknown
}

const OUTBOX_LOCK = 'aphel-offline-outbox'
const OUTBOX_CHANNEL = 'aphel-offline-outbox'
const IN_FLIGHT_GRACE_MS = 2 * 60_000
const SAFE_REPLAY_WINDOW_MS = 23 * 3600_000
const BASE_BACKOFF_MS = 5_000
const MAX_BACKOFF_MS = 5 * 60_000

const EMPTY_SNAPSHOT: OutboxSnapshot = { pending: [], conflicts: [] }

let snapshot: OutboxSnapshot = EMPTY_SNAPSHOT
const snapshotListeners = new Set<() => void>()
const deliveryListeners = new Set<(delivery: OutboxDelivery) => void>()
let channel: BroadcastChannel | null = null

function newId(): string {
  return crypto.randomUUID()
}

function sameOwner(item: OutboxItem, owner: OfflineOwner): boolean {
  return item.userId === owner.userId
}

function byCreation(left: OutboxItem, right: OutboxItem): number {
  return left.createdAt - right.createdAt
}

function backoff(attempts: number): number {
  const delay = Math.min(MAX_BACKOFF_MS, BASE_BACKOFF_MS * 2 ** Math.max(0, attempts - 1))
  return Date.now() + delay + Math.floor(Math.random() * 1_000)
}

function toOutboxError(error: unknown): OutboxError {
  if (isApiError(error)) return { status: error.status, code: error.code, message: error.message }
  return { status: 0, code: 'unknown_error', message: error instanceof Error ? error.message : String(error) }
}

async function putItem(item: OutboxItem): Promise<void> {
  await withStores(OUTBOX_STORE, 'readwrite', (transaction) => {
    transaction.objectStore(OUTBOX_STORE).put(item)
  })
}

async function getItem(id: string): Promise<OutboxItem | undefined> {
  return withStores(OUTBOX_STORE, 'readonly', (transaction) =>
    requestResult<OutboxItem | undefined>(transaction.objectStore(OUTBOX_STORE).get(id)),
  )
}

async function deleteItem(id: string): Promise<void> {
  await withStores(OUTBOX_STORE, 'readwrite', (transaction) => {
    transaction.objectStore(OUTBOX_STORE).delete(id)
  })
}

async function listItems(): Promise<OutboxItem[]> {
  const items = await withStores(OUTBOX_STORE, 'readonly', (transaction) =>
    requestResult<OutboxItem[]>(transaction.objectStore(OUTBOX_STORE).getAll()),
  )
  return items.sort(byCreation)
}

function emitSnapshot(next: OutboxSnapshot): void {
  snapshot = next
  for (const listener of snapshotListeners) listener()
}

export async function refreshOutboxSnapshot(): Promise<void> {
  const owner = getCurrentOwner()
  if (!owner || !isIndexedDbAvailable()) {
    emitSnapshot(EMPTY_SNAPSHOT)
    return
  }
  const items = (await listItems().catch(() => [])).filter((item) => sameOwner(item, owner))
  emitSnapshot({
    pending: items.filter((item) => item.status === 'pending'),
    conflicts: items.filter((item) => item.status === 'conflict'),
  })
}

function announceChange(): void {
  void refreshOutboxSnapshot()
  channel?.postMessage('changed')
}

export function connectOutboxChannel(): () => void {
  if (typeof BroadcastChannel === 'undefined' || channel) return () => undefined
  const opened = new BroadcastChannel(OUTBOX_CHANNEL)
  opened.onmessage = () => void refreshOutboxSnapshot()
  channel = opened
  return () => {
    opened.close()
    if (channel === opened) channel = null
  }
}

export function resetOutboxSnapshot(): void {
  emitSnapshot(EMPTY_SNAPSHOT)
}

function subscribeToSnapshot(listener: () => void): () => void {
  snapshotListeners.add(listener)
  return () => {
    snapshotListeners.delete(listener)
  }
}

export function useOutboxSnapshot(): OutboxSnapshot {
  return useSyncExternalStore(subscribeToSnapshot, () => snapshot, () => EMPTY_SNAPSHOT)
}

export function subscribeToOutboxDelivery(listener: (delivery: OutboxDelivery) => void): () => void {
  deliveryListeners.add(listener)
  return () => {
    deliveryListeners.delete(listener)
  }
}

function emitDelivery(delivery: OutboxDelivery): void {
  for (const listener of deliveryListeners) listener(delivery)
}

function send<T>(item: Pick<OutboxItem, 'path' | 'body' | 'idempotencyKey'>): Promise<T> {
  return api.post<T>(item.path, item.body, { headers: { [IDEMPOTENCY_KEY_HEADER]: item.idempotencyKey } })
}

function normalizeBody(body: unknown): unknown {
  return body === undefined ? undefined : (JSON.parse(JSON.stringify(body)) as unknown)
}

function createItem(submission: OutboxSubmission, owner: OfflineOwner): OutboxItem {
  const now = Date.now()
  return {
    id: newId(),
    idempotencyKey: newId(),
    kind: submission.kind,
    path: submission.path,
    body: normalizeBody(submission.body),
    label: submission.label,
    userId: owner.userId,
    createdAt: now,
    status: 'pending',
    attempts: 0,
    nextAttemptAt: now,
    firstAttemptAt: null,
    uncertain: false,
    error: null,
  }
}

async function enqueue(item: OutboxItem): Promise<OutboxResult<never>> {
  await putItem(item)
  announceChange()
  return { status: 'queued', id: item.id }
}

function persistInFlight(item: OutboxItem): Promise<boolean> {
  const now = Date.now()
  return putItem({ ...item, firstAttemptAt: now, uncertain: true, nextAttemptAt: now + IN_FLIGHT_GRACE_MS }).then(
    () => true,
    () => false,
  )
}

export async function submitWithOutbox<T>(submission: OutboxSubmission): Promise<OutboxResult<T>> {
  const owner = getCurrentOwner()
  const item = owner ? createItem(submission, owner) : null
  const persisted = item && isIndexedDbAvailable() ? await persistInFlight(item) : false

  if (!item || !persisted) {
    const data = await send<T>({
      path: submission.path,
      body: submission.body,
      idempotencyKey: item?.idempotencyKey ?? newId(),
    })
    return { status: 'sent', data }
  }

  if (!isOnline()) return enqueue(item)

  const attemptedAt = Date.now()
  try {
    const data = await send<T>(item)
    await deleteItem(item.id).catch(() => undefined)
    announceChange()
    return { status: 'sent', data }
  } catch (error) {
    if (isNetworkFailure(error)) {
      return enqueue({
        ...item,
        attempts: 1,
        firstAttemptAt: attemptedAt,
        uncertain: true,
        nextAttemptAt: Date.now(),
        error: toOutboxError(error),
      })
    }
    await deleteItem(item.id).catch(() => undefined)
    announceChange()
    throw error
  }
}

type AttemptOutcome = 'delivered' | 'stop' | 'next'

function isRetryable(error: unknown): boolean {
  if (!isApiError(error)) return true
  if (error.code === IDEMPOTENCY_KEY_IN_PROGRESS) return true
  return error.status >= 500 || error.status === 429 || error.status === 408
}

async function attempt(item: OutboxItem): Promise<AttemptOutcome> {
  const now = Date.now()
  if (item.uncertain && item.firstAttemptAt !== null && now - item.firstAttemptAt > SAFE_REPLAY_WINDOW_MS) {
    await putItem({
      ...item,
      status: 'conflict',
      error: { status: 0, code: REPLAY_WINDOW_EXPIRED, message: REPLAY_WINDOW_EXPIRED },
    })
    return 'next'
  }
  const claimed: OutboxItem = {
    ...item,
    attempts: item.attempts + 1,
    firstAttemptAt: item.firstAttemptAt ?? now,
    nextAttemptAt: now + IN_FLIGHT_GRACE_MS,
  }
  await putItem(claimed)
  try {
    const data = await send<unknown>(claimed)
    await deleteItem(claimed.id)
    emitDelivery({ item: claimed, data })
    return 'delivered'
  } catch (error) {
    const failure = toOutboxError(error)
    if (isNetworkFailure(error)) {
      await putItem({ ...claimed, uncertain: true, nextAttemptAt: backoff(claimed.attempts), error: failure })
      return 'stop'
    }
    if (isApiError(error) && error.status === 401) {
      await putItem({ ...claimed, nextAttemptAt: Date.now(), error: failure })
      return 'stop'
    }
    if (isRetryable(error)) {
      const inProgress = isApiError(error) && error.code === IDEMPOTENCY_KEY_IN_PROGRESS
      await putItem({
        ...claimed,
        uncertain: claimed.uncertain || inProgress,
        nextAttemptAt: backoff(claimed.attempts),
        error: failure,
      })
      return 'next'
    }
    await putItem({ ...claimed, status: 'conflict', error: failure })
    return 'next'
  }
}

async function drain(owner: OfflineOwner, force: boolean): Promise<void> {
  const queued = (await listItems()).filter((item) => item.status === 'pending' && sameOwner(item, owner))
  for (const listed of queued) {
    if (!isOnline() || getCurrentOwner()?.userId !== owner.userId) return
    const item = await getItem(listed.id)
    if (!item || item.status !== 'pending' || (!force && item.nextAttemptAt > Date.now())) continue
    const outcome = await attempt(item)
    announceChange()
    if (outcome === 'stop') return
  }
}

export interface FlushOptions {
  force?: boolean
}

export async function flushOutbox(options: FlushOptions = {}): Promise<void> {
  const owner = getCurrentOwner()
  if (!owner || !isOnline() || !isIndexedDbAvailable()) return
  await withExclusiveLock(OUTBOX_LOCK, () => drain(owner, options.force ?? false)).catch(() => undefined)
  await refreshOutboxSnapshot()
}

export async function retryOutboxItem(id: string): Promise<void> {
  const item = await getItem(id)
  if (!item) return
  await putItem({
    ...item,
    idempotencyKey: newId(),
    status: 'pending',
    attempts: 0,
    nextAttemptAt: Date.now(),
    firstAttemptAt: null,
    uncertain: false,
    error: null,
  })
  announceChange()
  await flushOutbox()
}

export async function discardOutboxItem(id: string): Promise<void> {
  await deleteItem(id)
  announceChange()
}

const TEXT_FIELDS = ['title', 'body', 'description', 'notes'] as const

export function outboxItemText(item: OutboxItem): string {
  if (typeof item.body !== 'object' || item.body === null) return ''
  const body = item.body as Record<string, unknown>
  return TEXT_FIELDS.map((field) => body[field])
    .filter((value): value is string => typeof value === 'string' && value.trim() !== '')
    .join('\n\n')
}
