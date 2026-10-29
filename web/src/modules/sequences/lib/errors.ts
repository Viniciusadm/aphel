import type { TFunction } from 'i18next'
import { isApiError } from '@/api'
import { errorMessage } from '@/components/feedback'
import { toast } from '@/components/ui/toast-store'
import { SEQUENCE_ERROR_CODES, type SequenceErrorCode } from '../constants'

function isSequenceErrorCode(code: string): code is SequenceErrorCode {
  return (SEQUENCE_ERROR_CODES as readonly string[]).includes(code)
}

export function sequenceErrorMessage(t: TFunction<'sequences'>, error: unknown): string {
  if (isApiError(error) && isSequenceErrorCode(error.code)) return t(`errors.${error.code}`)
  return errorMessage(error)
}

export function notifySequenceError(t: TFunction<'sequences'>, error: unknown, title?: string): number {
  return toast({
    title: title ?? t('toast.errorTitle'),
    description: sequenceErrorMessage(t, error),
    variant: 'destructive',
  })
}
