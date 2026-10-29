export const LOCALES = ['pt-BR', 'en'] as const

export const CURRENCIES = ['BRL', 'USD', 'EUR', 'GBP', 'ARS', 'CLP', 'COP', 'MXN', 'PEN', 'UYU', 'PYG', 'CAD', 'AUD', 'JPY', 'CHF'] as const

export const DEFAULT_TIMEZONE = 'America/Sao_Paulo'
export const DEFAULT_CURRENCY = 'BRL'
export const DEFAULT_LOCALE = 'pt-BR'

const FALLBACK_TIMEZONES = [
  'America/Sao_Paulo',
  'America/Manaus',
  'America/Belem',
  'America/Fortaleza',
  'America/Recife',
  'America/Bahia',
  'America/Cuiaba',
  'America/Porto_Velho',
  'America/Rio_Branco',
  'America/Noronha',
  'America/Argentina/Buenos_Aires',
  'America/Santiago',
  'America/Bogota',
  'America/Lima',
  'America/Mexico_City',
  'America/New_York',
  'America/Chicago',
  'America/Los_Angeles',
  'Europe/Lisbon',
  'Europe/London',
  'Europe/Madrid',
  'Europe/Paris',
  'Europe/Berlin',
  'Asia/Tokyo',
  'Australia/Sydney',
  'UTC',
]

export function timezoneOptions(current?: string | null): string[] {
  let zones = FALLBACK_TIMEZONES
  try {
    const supported = Intl.supportedValuesOf?.('timeZone')
    if (supported && supported.length > 0) zones = [...supported, 'UTC']
  } catch {
    zones = FALLBACK_TIMEZONES
  }
  const unique = new Set(zones)
  if (current) unique.add(current)
  return [...unique].sort((first, second) => first.localeCompare(second))
}

export function currencyLabel(code: string, locale: string): string {
  try {
    const name = new Intl.DisplayNames([locale], { type: 'currency' }).of(code)
    return name && name !== code ? `${code} — ${name}` : code
  } catch {
    return code
  }
}

export function currencyOptions(current?: string | null): string[] {
  const options: string[] = [...CURRENCIES]
  if (current && !options.includes(current)) options.push(current)
  return options
}

export function browserTimezone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || DEFAULT_TIMEZONE
  } catch {
    return DEFAULT_TIMEZONE
  }
}
