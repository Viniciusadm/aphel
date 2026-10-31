import { Loader2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Badge, type BadgeTone } from '@/components/ui/badge'
import type { MailboxStatus } from '../types'

const VARIANTS: Record<MailboxStatus, BadgeTone> = {
  active: 'success',
  syncing: 'primary',
  error: 'destructive',
  disconnected: 'warning',
}

export function MailboxStatusBadge({ status }: { status: MailboxStatus }) {
  const { t } = useTranslation('email')
  return (
    <Badge tone={VARIANTS[status]}>
      {status === 'syncing' && <Loader2 className="size-3 animate-spin" aria-hidden="true" />}
      {t(`mailboxStatus.${status}`)}
    </Badge>
  )
}
