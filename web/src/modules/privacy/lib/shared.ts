import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { notifyError } from '@/components/feedback'
import { toast } from '@/components/ui/toast-store'
import { getRecordResource } from '@/sdk'
export const PRIVACY_ERROR_CODES = [
  'invalid_subject_term',
  'privacy_targets_empty',
  'privacy_request_not_found',
  'privacy_request_not_pending',
  'privacy_export_unavailable',
  'retention_policy_not_found',
  'retention_preview_required',
  'retention_policy_limit',
] as const
type PrivacyErrorCode = (typeof PRIVACY_ERROR_CODES)[number]

function isPrivacyErrorCode(code: string): code is PrivacyErrorCode {
  return PRIVACY_ERROR_CODES.includes(code as PrivacyErrorCode)
}

export function usePrivacyErrors() {
  const { t } = useTranslation('privacy')
  return (error: unknown) => {
    if (isApiError(error) && isPrivacyErrorCode(error.code)) {
      toast({ title: t('errors.title'), description: t(`errors.codes.${error.code}`), variant: 'destructive' })
      return
    }
    notifyError(error)
  }
}

export function recordHref(recordType: string, id: string): string | null {
  return getRecordResource(recordType)?.detailPath?.(id) ?? null
}

export function addDays(days: number): string {
  const date = new Date()
  date.setDate(date.getDate() + days)
  const pad = (value: number) => String(value).padStart(2, '0')
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
}

export function endOfDayIso(date: string): string | undefined {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) return undefined
  const parsed = new Date(`${date}T23:59:59`)
  return Number.isNaN(parsed.getTime()) ? undefined : parsed.toISOString()
}

export function localizedLabel(labels: Record<string, string>, locale: string, fallback: string): string {
  return labels[locale] ?? labels[locale.split('-')[0]] ?? labels.en ?? Object.values(labels)[0] ?? fallback
}
