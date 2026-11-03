import { isApiError } from '@/api'

export const KNOWN_ERRORS = [
  'whatsapp_window_closed',
  'messaging_channel_inactive',
  'messaging_provider_unavailable',
  'messaging_invalid_reply',
  'messaging_invalid_assignee',
  'messaging_record_not_accessible',
  'messaging_channel_in_use',
  'messaging_channel_limit_reached',
  'messaging_acting_user_unavailable',
  'whatsapp_auth_failed',
  'imap_auth_failed',
  'smtp_auth_failed',
  'smtp_rejected',
]

export function errorText(t: (key: string) => string, error: unknown): string | null {
  return isApiError(error) && KNOWN_ERRORS.includes(error.code) ? t(`errors.${error.code}`) : null
}
