import { activeLocale } from '@/lib/locale'

interface ZonedParts {
  year: number
  month: number
  day: number
  hour: number
  minute: number
  second: number
}

const partFormatters = new Map<string, Intl.DateTimeFormat>()

function partFormatter(timeZone: string): Intl.DateTimeFormat {
  let formatter = partFormatters.get(timeZone)
  if (!formatter) {
    formatter = new Intl.DateTimeFormat('en-US', {
      timeZone,
      hourCycle: 'h23',
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    })
    partFormatters.set(timeZone, formatter)
  }
  return formatter
}

function zonedParts(date: Date, timeZone: string): ZonedParts {
  const values: Record<string, number> = {}
  for (const part of partFormatter(timeZone).formatToParts(date)) {
    if (part.type !== 'literal') values[part.type] = Number(part.value)
  }
  return {
    year: values.year,
    month: values.month,
    day: values.day,
    hour: values.hour % 24,
    minute: values.minute,
    second: values.second,
  }
}

function offsetMs(date: Date, timeZone: string): number {
  const parts = zonedParts(date, timeZone)
  const asUtc = Date.UTC(parts.year, parts.month - 1, parts.day, parts.hour, parts.minute, parts.second)
  return asUtc - Math.floor(date.getTime() / 1000) * 1000
}

function pad(value: number): string {
  return String(value).padStart(2, '0')
}

function parseKey(key: string): [number, number, number] {
  const [year, month, day] = key.split('-').map(Number)
  return [year, month, day]
}

export function dayKey(date: Date, timeZone: string): string {
  const parts = zonedParts(date, timeZone)
  return `${parts.year}-${pad(parts.month)}-${pad(parts.day)}`
}

export function addDays(key: string, days: number): string {
  const [year, month, day] = parseKey(key)
  const date = new Date(Date.UTC(year, month - 1, day + days))
  return `${date.getUTCFullYear()}-${pad(date.getUTCMonth() + 1)}-${pad(date.getUTCDate())}`
}

export function zonedDayStart(key: string, timeZone: string): Date {
  const [year, month, day] = parseKey(key)
  const guess = Date.UTC(year, month - 1, day)
  const firstOffset = offsetMs(new Date(guess), timeZone)
  const candidate = guess - firstOffset
  const secondOffset = offsetMs(new Date(candidate), timeZone)
  return new Date(secondOffset === firstOffset ? candidate : guess - secondOffset)
}

function keyAsDate(key: string): Date {
  const [year, month, day] = parseKey(key)
  return new Date(Date.UTC(year, month - 1, day, 12))
}

export function formatDayLabel(key: string): { weekday: string; day: string; month: string } {
  const date = keyAsDate(key)
  const locale = activeLocale()
  return {
    weekday: new Intl.DateTimeFormat(locale, { weekday: 'short', timeZone: 'UTC' }).format(date),
    day: new Intl.DateTimeFormat(locale, { day: 'numeric', timeZone: 'UTC' }).format(date),
    month: new Intl.DateTimeFormat(locale, { month: 'short', timeZone: 'UTC' }).format(date),
  }
}

export function formatLongDay(key: string): string {
  return new Intl.DateTimeFormat(activeLocale(), {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    timeZone: 'UTC',
  }).format(keyAsDate(key))
}

export function formatRangeLabel(startKey: string, endKey: string): string {
  const formatter = new Intl.DateTimeFormat(activeLocale(), { day: 'numeric', month: 'short', timeZone: 'UTC' })
  return `${formatter.format(keyAsDate(startKey))} – ${formatter.format(keyAsDate(endKey))}`
}

export function formatTime(iso: string, timeZone: string): string {
  return new Intl.DateTimeFormat(activeLocale(), { hour: '2-digit', minute: '2-digit', timeZone }).format(
    new Date(iso),
  )
}

export function formatWhen(startsAt: string, endsAt: string, timeZone: string): string {
  const date = new Intl.DateTimeFormat(activeLocale(), {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    year: 'numeric',
    timeZone,
  }).format(new Date(startsAt))
  return `${date}, ${formatTime(startsAt, timeZone)} – ${formatTime(endsAt, timeZone)}`
}
