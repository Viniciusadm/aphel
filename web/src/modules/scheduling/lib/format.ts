import { activeLocale } from '@/lib/locale'

export function formatDateTimeRange(startsAt: string, endsAt: string, timeZone?: string): string {
  const start = new Date(startsAt)
  const end = new Date(endsAt)
  const date = new Intl.DateTimeFormat(activeLocale(), {
    weekday: 'short',
    day: '2-digit',
    month: 'short',
    year: 'numeric',
    timeZone,
  }).format(start)
  const time = new Intl.DateTimeFormat(activeLocale(), { hour: '2-digit', minute: '2-digit', timeZone })
  return `${date}, ${time.format(start)} – ${time.format(end)}`
}

export function publicBookingUrl(bookingPath: string): string {
  return `${window.location.origin}${bookingPath}`
}

export function availableTimeZones(): string[] {
  const intl = Intl as typeof Intl & { supportedValuesOf?: (key: string) => string[] }
  try {
    return intl.supportedValuesOf ? intl.supportedValuesOf('timeZone') : []
  } catch {
    return []
  }
}

export function isValidTimeZone(timeZone: string): boolean {
  try {
    new Intl.DateTimeFormat('en-US', { timeZone })
    return true
  } catch {
    return false
  }
}

export function browserTimeZone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'
}
