import { activeLocale } from '@/lib/locale'
import { publicUrl } from '@/sdk/app-base'

export function formatBookingDate(startsAt: string, timeZone?: string): string {
  return new Intl.DateTimeFormat(activeLocale(), {
    weekday: 'short',
    day: '2-digit',
    month: 'short',
    year: 'numeric',
    timeZone,
  }).format(new Date(startsAt))
}

export function formatBookingTimes(startsAt: string, endsAt: string, timeZone?: string): string {
  const time = new Intl.DateTimeFormat(activeLocale(), { hour: '2-digit', minute: '2-digit', timeZone })
  return `${time.format(new Date(startsAt))} – ${time.format(new Date(endsAt))}`
}

export function publicBookingUrl(bookingPath: string): string {
  return publicUrl(bookingPath)
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
