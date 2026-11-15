export function currencyName(code: string, locale: string): string {
  try {
    return new Intl.DisplayNames([locale], { type: 'currency' }).of(code) ?? code
  } catch {
    return code
  }
}

export function formatRate(value: unknown, locale: string): string {
  const number = Number(value)
  if (!Number.isFinite(number)) return ''
  return new Intl.NumberFormat(locale, { minimumFractionDigits: 2, maximumFractionDigits: 6 }).format(number)
}

const COMMON = ['USD', 'EUR', 'GBP', 'JPY', 'CNY', 'ARS', 'CLP', 'COP', 'MXN', 'PEN', 'UYU', 'PYG', 'BOB', 'CAD', 'AUD', 'CHF', 'INR', 'ZAR', 'BRL']

export function selectableCurrencies(locale: string): { code: string; label: string }[] {
  const supported = (Intl as unknown as { supportedValuesOf?: (key: string) => string[] }).supportedValuesOf
  const codes = (supported ? supported('currency') : COMMON).filter((code) => /^[A-Z]{3}$/.test(code))
  return codes
    .map((code) => ({ code, label: `${currencyName(code, locale)} (${code})` }))
    .sort((left, right) => left.label.localeCompare(right.label, locale))
}
