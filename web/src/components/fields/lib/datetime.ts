const LOCAL_INPUT = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})(?::(\d{2}))?$/

interface WallTime {
  year: number
  month: number
  day: number
  hour: number
  minute: number
  second: number
}

function safeTimeZone(timeZone: string): string {
  try {
    new Intl.DateTimeFormat('en-US', { timeZone })
    return timeZone
  } catch {
    return 'UTC'
  }
}

function wallTime(date: Date, timeZone: string): WallTime {
  const parts = new Intl.DateTimeFormat('en-US', {
    timeZone: safeTimeZone(timeZone),
    hourCycle: 'h23',
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  }).formatToParts(date)
  const part = (type: Intl.DateTimeFormatPartTypes) => Number(parts.find((item) => item.type === type)?.value ?? 0)
  return {
    year: part('year'),
    month: part('month'),
    day: part('day'),
    hour: part('hour'),
    minute: part('minute'),
    second: part('second'),
  }
}

function timeZoneOffset(date: Date, timeZone: string): number {
  const wall = wallTime(date, timeZone)
  const asUtc = Date.UTC(wall.year, wall.month - 1, wall.day, wall.hour, wall.minute, wall.second)
  return asUtc - Math.floor(date.getTime() / 1000) * 1000
}

function pad(value: number, length = 2): string {
  return String(value).padStart(length, '0')
}

export function utcToZonedInput(value: unknown, timeZone: string): string {
  if (typeof value !== 'string' || value === '') return ''
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return ''
  const wall = wallTime(date, timeZone)
  return `${pad(wall.year, 4)}-${pad(wall.month)}-${pad(wall.day)}T${pad(wall.hour)}:${pad(wall.minute)}`
}

export function zonedInputToUtc(local: string, timeZone: string): string | null {
  const match = LOCAL_INPUT.exec(local)
  if (!match) return null
  const [, year, month, day, hour, minute, second] = match
  const wall = Date.UTC(Number(year), Number(month) - 1, Number(day), Number(hour), Number(minute), Number(second ?? 0))
  if (Number.isNaN(wall)) return null
  const firstGuess = wall - timeZoneOffset(new Date(wall), timeZone)
  const utc = wall - timeZoneOffset(new Date(firstGuess), timeZone)
  return new Date(utc).toISOString().replace(/\.\d{3}Z$/, 'Z')
}

export function timeZoneLabel(timeZone: string, locale: string, at: Date = new Date()): string {
  try {
    const parts = new Intl.DateTimeFormat(locale, { timeZone, timeZoneName: 'short' }).formatToParts(at)
    return parts.find((part) => part.type === 'timeZoneName')?.value ?? timeZone
  } catch {
    return timeZone
  }
}
