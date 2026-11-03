import { utcToZonedInput, zonedInputToUtc } from '@/components/fields/lib/datetime'
import i18n, { DEFAULT_LANGUAGE } from '@/i18n'

export const MINUTES_PER_DAY = 24 * 60
export const MONTH_GRID_DAYS = 42
const KEY_PATTERN = /^(\d{4})-(\d{2})-(\d{2})$/

function keyToUtc(key: string): number {
  const match = KEY_PATTERN.exec(key)
  if (!match) return Number.NaN
  return Date.UTC(Number(match[1]), Number(match[2]) - 1, Number(match[3]))
}

function utcToKey(value: number): string {
  return new Date(value).toISOString().slice(0, 10)
}

export function isDayKey(value: unknown): value is string {
  return typeof value === 'string' && KEY_PATTERN.test(value) && !Number.isNaN(keyToUtc(value))
}

export function addDays(key: string, days: number): string {
  return utcToKey(keyToUtc(key) + days * 86_400_000)
}

export function addMonths(key: string, months: number): string {
  const date = new Date(keyToUtc(key))
  const target = new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth() + months, 1))
  const lastDay = new Date(Date.UTC(target.getUTCFullYear(), target.getUTCMonth() + 1, 0)).getUTCDate()
  target.setUTCDate(Math.min(date.getUTCDate(), lastDay))
  return utcToKey(target.getTime())
}

export function weekday(key: string): number {
  return new Date(keyToUtc(key)).getUTCDay()
}

const SUNDAY = 0
const MONDAY = 1
const DAYS_PER_WEEK = 7
const REGION_WEEK_STARTS: Record<string, number> = { BR: MONDAY }

interface LocaleWeekInfo {
  getWeekInfo?: () => { firstDay: number }
  weekInfo?: { firstDay: number }
}

export function weekStartFor(locale: string): number {
  try {
    const intl = new Intl.Locale(locale).maximize()
    const override = intl.region ? REGION_WEEK_STARTS[intl.region] : undefined
    if (override !== undefined) return override
    const info = intl as unknown as LocaleWeekInfo
    const firstDay = info.getWeekInfo?.().firstDay ?? info.weekInfo?.firstDay
    if (firstDay !== undefined) return firstDay % DAYS_PER_WEEK
    return intl.region === 'US' ? SUNDAY : MONDAY
  } catch {
    return MONDAY
  }
}

export function defaultWeekStart(): number {
  return weekStartFor(i18n.resolvedLanguage ?? i18n.language ?? DEFAULT_LANGUAGE)
}

export function startOfWeek(key: string, weekStartsOn: number = defaultWeekStart()): string {
  return addDays(key, -((weekday(key) - weekStartsOn + DAYS_PER_WEEK) % DAYS_PER_WEEK))
}

export function startOfMonth(key: string): string {
  return `${key.slice(0, 8)}01`
}

export function sameMonth(left: string, right: string): boolean {
  return left.slice(0, 7) === right.slice(0, 7)
}

export function daysBetween(from: string, to: string): string[] {
  const days: string[] = []
  for (let key = from; key < to; key = addDays(key, 1)) days.push(key)
  return days
}

export function monthGridStart(key: string, weekStartsOn: number = defaultWeekStart()): string {
  return startOfWeek(startOfMonth(key), weekStartsOn)
}

export function monthGrid(key: string, weekStartsOn: number = defaultWeekStart()): string[] {
  const start = monthGridStart(key, weekStartsOn)
  return Array.from({ length: MONTH_GRID_DAYS }, (_, index) => addDays(start, index))
}

export function zonedDayKey(instant: string | Date, timeZone: string): string | null {
  const local = utcToZonedInput(typeof instant === 'string' ? instant : instant.toISOString(), timeZone)
  return local ? local.slice(0, 10) : null
}

export function zonedMinutes(instant: string, timeZone: string): number | null {
  const local = utcToZonedInput(instant, timeZone)
  if (!local) return null
  return Number(local.slice(11, 13)) * 60 + Number(local.slice(14, 16))
}

function pad(value: number): string {
  return String(value).padStart(2, '0')
}

export function zonedInstant(key: string, minutes: number, timeZone: string): string | null {
  const clamped = Math.max(0, Math.min(MINUTES_PER_DAY - 1, Math.round(minutes)))
  return zonedInputToUtc(`${key}T${pad(Math.floor(clamped / 60))}:${pad(clamped % 60)}`, timeZone)
}

export function zonedDayStart(key: string, timeZone: string): string {
  return zonedInstant(key, 0, timeZone) ?? `${key}T00:00:00Z`
}

export function todayKey(timeZone: string, now: Date = new Date()): string {
  return zonedDayKey(now, timeZone) ?? utcToKey(now.getTime())
}

function keyDate(key: string): Date {
  return new Date(keyToUtc(key))
}

export function formatMonthTitle(key: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, { timeZone: 'UTC', month: 'long', year: 'numeric' }).format(keyDate(key))
}

export function formatDayTitle(key: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    timeZone: 'UTC',
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  }).format(keyDate(key))
}

export function formatShortDay(key: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, { timeZone: 'UTC', day: 'numeric', month: 'short' }).format(keyDate(key))
}

export function formatWeekdayShort(key: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, { timeZone: 'UTC', weekday: 'short' }).format(keyDate(key))
}

export function formatDayNumber(key: string): string {
  return String(keyDate(key).getUTCDate())
}

export function formatMinutes(minutes: number, locale: string): string {
  return new Intl.DateTimeFormat(locale, { timeZone: 'UTC', hour: '2-digit', minute: '2-digit' }).format(
    new Date(Date.UTC(2000, 0, 1, Math.floor(minutes / 60), minutes % 60)),
  )
}

export function groupByDay<T>(items: T[], dayOf: (item: T) => string | null): Map<string, T[]> {
  const groups = new Map<string, T[]>()
  for (const item of items) {
    const key = dayOf(item)
    if (!key) continue
    const list = groups.get(key)
    if (list) list.push(item)
    else groups.set(key, [item])
  }
  return groups
}

export interface DaySegment {
  day: string
  start: number
  end: number
}

export function spanSegments(
  startInstant: string,
  durationMinutes: number,
  timeZone: string,
  maxDays: number = DAYS_PER_WEEK * 6,
): DaySegment[] {
  const day = zonedDayKey(startInstant, timeZone)
  const startMinutes = zonedMinutes(startInstant, timeZone)
  if (!day || startMinutes === null) return []
  const segments: DaySegment[] = []
  let remaining = Math.max(0, durationMinutes)
  let current = day
  let offset = startMinutes
  do {
    const end = Math.min(MINUTES_PER_DAY, offset + remaining)
    segments.push({ day: current, start: offset, end })
    remaining -= end - offset
    current = addDays(current, 1)
    offset = 0
  } while (remaining > 0 && segments.length < maxDays)
  return segments
}

export function groupBySpan<T>(items: T[], daysOf: (item: T) => string[]): Map<string, T[]> {
  const groups = new Map<string, T[]>()
  for (const item of items) {
    for (const key of new Set(daysOf(item))) {
      const list = groups.get(key)
      if (list) list.push(item)
      else groups.set(key, [item])
    }
  }
  return groups
}
