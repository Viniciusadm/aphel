import { dayKey } from '@/sdk'
export type DueState = 'overdue' | 'today' | 'upcoming' | 'none'

export function dueState(dueAt: string | null | undefined, timeZone: string, now: Date = new Date()): DueState {
  if (!dueAt) return 'none'
  const due = new Date(dueAt)
  if (Number.isNaN(due.getTime())) return 'none'
  const dueDay = dayKey(due, timeZone)
  const today = dayKey(now, timeZone)
  if (dueDay === today) return due.getTime() < now.getTime() ? 'overdue' : 'today'
  if (dueDay !== null && today !== null && dueDay < today) return 'overdue'
  return 'upcoming'
}

export interface DueGroup<T> {
  state: DueState
  items: T[]
}

export function groupByDue<T extends { due_at?: string | null }>(
  items: T[],
  timeZone: string,
  now: Date = new Date(),
): DueGroup<T>[] {
  const order: DueState[] = ['overdue', 'today', 'upcoming', 'none']
  return order
    .map((state) => ({ state, items: items.filter((item) => dueState(item.due_at, timeZone, now) === state) }))
    .filter((group) => group.items.length > 0)
}
