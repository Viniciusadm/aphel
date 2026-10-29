import { Loader2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import type { MailboxStatus } from '../types'

const VARIANTS: Record<MailboxStatus, BadgeProps['variant']> = {
  active: 'success',
  syncing: 'secondary',
  error: 'destructive',
  disconnected: 'warning',
}

export function MailboxStatusBadge({ status }: { status: MailboxStatus }) {
  const { t } = useTranslation('email')
  return (
    <Badge variant={VARIANTS[status]}>
      {status === 'syncing' && <Loader2 className="size-3 animate-spin" aria-hidden="true" />}
      {t(`mailboxStatus.${status}`)}
    </Badge>
  )
}
