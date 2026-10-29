import { isApiError } from '@/api'
import { notifyError } from '@/components/feedback'
import { toast } from '@/components/ui/toast-store'
import i18n from '@/i18n'

const PROVIDER_ERROR_CODES = [
  'mail_provider_authentication',
  'mail_provider_connection',
  'mail_provider_rejected',
  'mail_provider_unsupported',
  'mail_provider_internal',
] as const

type ProviderErrorCode = (typeof PROVIDER_ERROR_CODES)[number]

function isProviderErrorCode(code: string): code is ProviderErrorCode {
  return (PROVIDER_ERROR_CODES as readonly string[]).includes(code)
}

export function providerErrorTitle(error: unknown): string | null {
  if (!isApiError(error)) return null
  if (isProviderErrorCode(error.code)) return i18n.t(`errors.${error.code}`, { ns: 'email' })
  if (error.code.startsWith('mail_provider_')) return i18n.t('errors.mail_provider_internal', { ns: 'email' })
  return null
}

export function notifyEmailError(error: unknown) {
  const title = providerErrorTitle(error)
  if (!title || !isApiError(error)) {
    notifyError(error)
    return
  }
  toast({ title, description: error.message || undefined, variant: 'destructive' })
}

const OAUTH_ERROR_CODES = ['access_denied', 'invalid_state', 'exchange_failed', 'already_connected'] as const

type OAuthErrorCode = (typeof OAUTH_ERROR_CODES)[number]

export function oauthErrorMessage(code: string): string {
  if ((OAUTH_ERROR_CODES as readonly string[]).includes(code)) {
    return i18n.t(`mailboxes.oauthErrors.${code as OAuthErrorCode}`, { ns: 'email' })
  }
  return i18n.t('mailboxes.oauthErrors.generic', { ns: 'email', code })
}
