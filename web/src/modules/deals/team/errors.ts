import type { TFunction } from 'i18next'
import { isApiError } from '@/api'

export function dealTeamErrorMessage(error: unknown, t: TFunction<'deals', 'team'>): string {
  if (!isApiError(error)) return t('errors.generic')
  switch (error.code) {
    case 'permission_denied':
    case 'insufficient_scope':
      return t('errors.forbidden')
    case 'deal_not_found':
    case 'deal_member_not_found':
    case 'deal_member_role_not_found':
      return t('errors.notFound')
    case 'deal_member_exists':
      return t('errors.exists')
    case 'deal_members_limit_reached':
      return t('errors.limit')
    case 'deal_credit_exceeds_total':
      return t('errors.exceeds', { available: availablePercent(error.details) })
    case 'deal_member_role_key_taken':
      return t('errors.roleKeyTaken')
    case 'version_conflict':
      return t('errors.versionConflict')
    case 'validation_error':
      return t('errors.invalid')
    default:
      return t('errors.generic')
  }
}

function availablePercent(details: unknown): string {
  if (typeof details !== 'object' || details === null) return '0'
  const value = (details as Record<string, unknown>).available_percent
  return typeof value === 'string' ? value : '0'
}

export function parsePercent(value: string | null | undefined): number {
  const parsed = Number(value ?? '0')
  return Number.isFinite(parsed) ? parsed : 0
}

export function formatPercent(value: string | number | null | undefined, locale: string): string {
  const number = typeof value === 'number' ? value : parsePercent(value)
  return `${number.toLocaleString(locale, { maximumFractionDigits: 2 })}%`
}

export function toPercentInput(text: string): string {
  return text.trim().replace(/\s/g, '').replace(',', '.')
}
