import type { TimelineItem } from './types'

export interface TimelineDayGroup {
  key: string
  items: TimelineItem[]
}

export type TimelineDayLabel = { kind: 'today' } | { kind: 'yesterday' } | { kind: 'date'; text: string }

const DAY_KEY_PATTERN = /^(\d{4})-(\d{2})-(\d{2})$/

export function dayKey(value: string | Date, timeZone: string): string | null {
  const date = typeof value === 'string' ? new Date(value) : value
  if (Number.isNaN(date.getTime())) return null
  const parts = new Intl.DateTimeFormat('en-CA', {
    timeZone,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }).formatToParts(date)
  const part = (type: Intl.DateTimeFormatPartTypes) => parts.find((item) => item.type === type)?.value ?? ''
  return `${part('year')}-${part('month')}-${part('day')}`
}

function dayKeyToUtcDate(key: string): Date {
  const match = DAY_KEY_PATTERN.exec(key)
  if (!match) return new Date(Number.NaN)
  return new Date(Date.UTC(Number(match[1]), Number(match[2]) - 1, Number(match[3])))
}

export function shiftDayKey(key: string, days: number): string {
  const date = dayKeyToUtcDate(key)
  date.setUTCDate(date.getUTCDate() + days)
  return date.toISOString().slice(0, 10)
}

function timestamp(item: TimelineItem) {
  return new Date(item.occurred_at).getTime()
}

export function groupTimelineByDay(items: TimelineItem[], timeZone: string): TimelineDayGroup[] {
  const sorted = items
    .filter((item) => !Number.isNaN(timestamp(item)))
    .sort((left, right) => timestamp(right) - timestamp(left))
  const groups: TimelineDayGroup[] = []
  for (const item of sorted) {
    const key = dayKey(item.occurred_at, timeZone)
    if (!key) continue
    const last = groups.at(-1)
    if (last?.key === key) last.items.push(item)
    else groups.push({ key, items: [item] })
  }
  return groups
}

export function timelineDayLabel(key: string, now: Date, timeZone: string, locale: string): TimelineDayLabel {
  const today = dayKey(now, timeZone)
  if (key === today) return { kind: 'today' }
  if (today && key === shiftDayKey(today, -1)) return { kind: 'yesterday' }
  const sameYear = today?.slice(0, 4) === key.slice(0, 4)
  const text = new Intl.DateTimeFormat(locale, {
    timeZone: 'UTC',
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    ...(sameYear ? {} : { year: 'numeric' }),
  }).format(dayKeyToUtcDate(key))
  return { kind: 'date', text: text.charAt(0).toLocaleUpperCase(locale) + text.slice(1) }
}

export function formatTimelineTime(value: string, timeZone: string, locale: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return ''
  return new Intl.DateTimeFormat(locale, { timeZone, timeStyle: 'short' }).format(date)
}

export function filterTimelineItems(items: TimelineItem[], kinds: string[]): TimelineItem[] {
  if (kinds.length === 0) return items
  const selected = new Set(kinds)
  return items.filter((item) => selected.has(item.kind))
}
