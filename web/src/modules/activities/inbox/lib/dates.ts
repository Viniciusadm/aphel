import { utcToZonedInput, zonedInputToUtc } from '@/components/fields/lib/datetime'

export const DEFAULT_POSTPONE_TIME = '09:00'

function shiftLocalDate(local: string, days: number): string {
  const [date, time] = local.split('T')
  const [year, month, day] = date.split('-').map(Number)
  const shifted = new Date(Date.UTC(year, month - 1, day + days))
  const pad = (value: number, length = 2) => String(value).padStart(length, '0')
  return `${pad(shifted.getUTCFullYear(), 4)}-${pad(shifted.getUTCMonth() + 1)}-${pad(shifted.getUTCDate())}T${time}`
}

export function postponedDueAt(
  dueAt: string | null | undefined,
  days: number,
  timeZone: string,
  now: Date = new Date(),
): string | null {
  if (dueAt) {
    const local = utcToZonedInput(dueAt, timeZone)
    if (local) return zonedInputToUtc(shiftLocalDate(local, days), timeZone)
  }
  const today = utcToZonedInput(now.toISOString(), timeZone)
  if (!today) return null
  const base = `${today.split('T')[0]}T${DEFAULT_POSTPONE_TIME}`
  return zonedInputToUtc(shiftLocalDate(base, days), timeZone)
}
