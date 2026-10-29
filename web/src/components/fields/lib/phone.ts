import {
  AsYouType,
  getCountries,
  getCountryCallingCode,
  isValidPhoneNumber,
  parsePhoneNumberFromString,
  validatePhoneNumberLength,
  type CountryCode,
} from 'libphonenumber-js'

export interface PhoneCountry {
  code: CountryCode
  dialCode: string
}

export const DEFAULT_PHONE_COUNTRY: CountryCode = 'BR'
const MAX_E164_DIGITS = 15

export const PHONE_COUNTRIES: PhoneCountry[] = getCountries().map((code) => ({
  code,
  dialCode: getCountryCallingCode(code),
}))

const COUNTRIES_BY_CODE = new Map(PHONE_COUNTRIES.map((country) => [country.code as string, country]))

export function findPhoneCountry(code: string | null | undefined): PhoneCountry | undefined {
  if (!code) return undefined
  return COUNTRIES_BY_CODE.get(code.toUpperCase())
}

export function resolvePhoneCountry(code: string | null | undefined): PhoneCountry {
  return findPhoneCountry(code) ?? findPhoneCountry(DEFAULT_PHONE_COUNTRY)!
}

export function limitNationalDigits(country: PhoneCountry, digits: string): string {
  let value = digits.slice(0, MAX_E164_DIGITS - country.dialCode.length)
  while (value.length > 0 && validatePhoneNumberLength(value, country.code) === 'TOO_LONG') {
    value = value.slice(0, -1)
  }
  return value
}

export interface PhoneParts {
  country: PhoneCountry
  national: string
}

function countryByDialCode(digits: string, fallback: PhoneCountry): PhoneParts | null {
  const candidates = PHONE_COUNTRIES.filter((country) => digits.startsWith(country.dialCode))
  if (candidates.length === 0) return null
  const country =
    candidates.find((candidate) => candidate.code === fallback.code) ??
    candidates.reduce((best, candidate) => (candidate.dialCode.length > best.dialCode.length ? candidate : best))
  return { country, national: digits.slice(country.dialCode.length) }
}

export function splitPhoneNumber(value: unknown, fallback: PhoneCountry): PhoneParts {
  if (typeof value !== 'string' || value.trim() === '') return { country: fallback, national: '' }
  const digits = value.replace(/\D/g, '')
  if (!value.trim().startsWith('+')) return { country: fallback, national: digits }
  const parsed = parsePhoneNumberFromString(`+${digits}`)
  if (parsed) {
    const country =
      findPhoneCountry(parsed.country) ?? (parsed.countryCallingCode === fallback.dialCode ? fallback : undefined)
    if (country) return { country, national: parsed.nationalNumber }
  }
  return countryByDialCode(digits, fallback) ?? { country: fallback, national: digits }
}

export function toE164(country: PhoneCountry, national: string): string | null {
  const digits = national.replace(/\D/g, '')
  if (!digits) return null
  const parsed = parsePhoneNumberFromString(digits, country.code)
  if (parsed && parsed.countryCallingCode === country.dialCode) return parsed.number
  const trimmed = digits.replace(/^0+/, '')
  if (!trimmed) return null
  return `+${country.dialCode}${trimmed.slice(0, MAX_E164_DIGITS - country.dialCode.length)}`
}

export function formatNationalNumber(country: PhoneCountry, digits: string): string {
  if (!digits) return ''
  return new AsYouType(country.code).input(digits)
}

export function isPhoneValid(value: unknown): boolean {
  return typeof value === 'string' && isValidPhoneNumber(value)
}

export function countryName(code: string, locale: string): string {
  try {
    return new Intl.DisplayNames([locale], { type: 'region' }).of(code) ?? code
  } catch {
    return code
  }
}
