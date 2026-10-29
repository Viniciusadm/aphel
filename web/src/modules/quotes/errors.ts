import { isApiError } from '@/api'
import { notifyError } from '@/components/feedback'
import i18n from '@/i18n'

const KNOWN = [
  'approval_required',
  'approval_pending',
  'quote_not_editable',
  'quote_not_current',
  'quote_expired',
  'quote_has_no_items',
  'quote_already_accepted',
  'quote_invalid_transition',
] as const

type KnownCode = (typeof KNOWN)[number]

function isKnown(code: string): code is KnownCode {
  return (KNOWN as readonly string[]).includes(code)
}

export function notifyQuoteError(error: unknown) {
  if (isApiError(error) && isKnown(error.code)) {
    notifyError(error, { title: i18n.t(`errors.${error.code}`, { ns: 'quotes' }) })
    return
  }
  notifyError(error)
}
