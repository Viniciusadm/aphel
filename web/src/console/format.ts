import i18n from '@/i18n'

export function formatBytes(bytes: number | null | undefined): string | null {
  if (bytes === null || bytes === undefined) return null
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let value = bytes
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit += 1
  }
  return `${new Intl.NumberFormat(i18n.language, { maximumFractionDigits: value < 10 && unit > 0 ? 1 : 0 }).format(value)} ${units[unit]}`
}

export function formatDateTime(value: string | null | undefined): string | null {
  if (!value) return null
  return new Intl.DateTimeFormat(i18n.language, { dateStyle: 'short', timeStyle: 'short' }).format(new Date(value))
}

const AGE_UNITS: [string, number][] = [
  ['day', 86_400],
  ['hour', 3_600],
  ['minute', 60],
  ['second', 1],
]

export function formatAge(seconds: number): string {
  const [unit, size] = AGE_UNITS.find(([, size]) => seconds >= size) ?? ['second', 1]
  return new Intl.NumberFormat(i18n.language, { style: 'unit', unit, unitDisplay: 'long', maximumFractionDigits: 0 }).format(
    Math.floor(seconds / size),
  )
}
