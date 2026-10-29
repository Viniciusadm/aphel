import { useTranslation } from 'react-i18next'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import type { SecurityEvent } from './api'

type Metadata = Record<string, unknown>

const FAILURE_REASONS = ['unknown_account', 'wrong_password', 'inactive_account', 'invalid_two_factor_code'] as const
type FailureReason = (typeof FAILURE_REASONS)[number]

function text(metadata: Metadata, key: string): string | undefined {
  const value = metadata[key]
  return typeof value === 'string' && value.trim() ? value : undefined
}

function count(metadata: Metadata, key: string): number | undefined {
  const value = metadata[key]
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined
}

export function useEventDetails() {
  const { t } = useTranslation('security')
  const formatting = useRecordFormatting()

  return (event: SecurityEvent): string | null => {
    const metadata: Metadata =
      typeof event.metadata === 'object' && event.metadata !== null ? (event.metadata as Metadata) : {}
    switch (event.kind) {
      case 'login_failed': {
        const reason = text(metadata, 'reason')
        return reason && FAILURE_REASONS.includes(reason as FailureReason)
          ? t(`log.details.reasons.${reason as FailureReason}`)
          : null
      }
      case 'login_locked': {
        const until = text(metadata, 'locked_until')
        return until ? t('log.details.lockedUntil', { date: formatDateTime(until, formatting) }) : null
      }
      case 'session_revoked': {
        if (text(metadata, 'scope') === 'others') {
          return t('log.details.sessionsRevoked', { count: count(metadata, 'count') ?? 0 })
        }
        const device = text(metadata, 'session_device')
        return device
          ? t('log.details.sessionRevoked', { device, ip: text(metadata, 'session_ip') ?? t('unknownIp') })
          : null
      }
      case 'password_changed': {
        const revoked = count(metadata, 'revoked_sessions')
        return revoked !== undefined ? t('log.details.passwordChanged', { count: revoked }) : null
      }
      case 'api_token_created':
      case 'api_token_revoked': {
        const name = text(metadata, 'token_name')
        return name ? t('log.details.apiToken', { name, prefix: text(metadata, 'token_prefix') ?? '' }) : null
      }
      case 'permission_changed': {
        if (text(metadata, 'change') === 'record_share') {
          const action = text(metadata, 'action')
          const key =
            action === 'unshared'
              ? 'log.details.recordUnshared'
              : action === 'shared'
                ? 'log.details.recordShared'
                : 'log.details.recordShareUpdated'
          return t(key, {
            name: text(metadata, 'grantee_name') ?? '—',
            access: t(text(metadata, 'access') === 'edit' ? 'log.details.shareAccessEdit' : 'log.details.shareAccessRead'),
          })
        }
        if (text(metadata, 'change') === 'member_role') {
          return t('log.details.memberRole', {
            from: text(metadata, 'old_role') ?? '—',
            to: text(metadata, 'new_role') ?? '—',
          })
        }
        const role = text(metadata, 'role_name')
        return role ? t('log.details.rolePermissions', { role }) : null
      }
      default:
        return null
    }
  }
}
