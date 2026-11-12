import { useContext, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { SessionContext } from '@/app/session/session-context'
import { intlLocale } from '@/lib/locale'

export interface RecordFormatting {
  locale: string
  timezone: string
  currency: string
}

export const DEFAULT_FORMATTING: Omit<RecordFormatting, 'locale'> = {
  timezone: 'America/Sao_Paulo',
  currency: 'BRL',
}

export function useRecordFormatting(): RecordFormatting {
  const session = useContext(SessionContext)
  const { i18n } = useTranslation()
  const organization = session?.me.organization
  const language = i18n.resolvedLanguage || i18n.language
  return useMemo(
    () => ({
      locale: language ? intlLocale(language) : intlLocale(organization?.locale ?? undefined),
      timezone: organization?.timezone ?? DEFAULT_FORMATTING.timezone,
      currency: organization?.currency ?? DEFAULT_FORMATTING.currency,
    }),
    [organization?.locale, organization?.timezone, organization?.currency, language],
  )
}

function toNumber(value: unknown): number | null {
  if (typeof value === 'number') return Number.isFinite(value) ? value : null
  if (typeof value === 'string' && value.trim() !== '') {
    const parsed = Number(value)
    return Number.isFinite(parsed) ? parsed : null
  }
  return null
}

export function formatNumber(value: unknown, formatting: RecordFormatting, decimals?: number | null): string {
  const number = toNumber(value)
  if (number === null) return ''
  return new Intl.NumberFormat(formatting.locale, {
    minimumFractionDigits: decimals ?? 0,
    maximumFractionDigits: decimals ?? 6,
  }).format(number)
}

export function formatCurrency(value: unknown, formatting: RecordFormatting, currency?: string | null): string {
  const number = toNumber(value)
  if (number === null) return ''
  return new Intl.NumberFormat(formatting.locale, {
    style: 'currency',
    currency: currency || formatting.currency,
  }).format(number)
}

export function formatDate(value: unknown, formatting: RecordFormatting): string {
  if (typeof value !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(value)) return ''
  const [year, month, day] = value.split('-').map(Number)
  return new Intl.DateTimeFormat(formatting.locale, { timeZone: 'UTC', dateStyle: 'short' }).format(
    new Date(Date.UTC(year, month - 1, day)),
  )
}

export function formatDateTime(value: unknown, formatting: RecordFormatting): string {
  if (typeof value !== 'string') return ''
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return ''
  return new Intl.DateTimeFormat(formatting.locale, {
    timeZone: formatting.timezone,
    dateStyle: 'short',
    timeStyle: 'short',
  }).format(date)
}
