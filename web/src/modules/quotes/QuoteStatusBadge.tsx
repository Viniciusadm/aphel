import { useTranslation } from 'react-i18next'
import { Badge, type BadgeTone } from '@/components/ui/badge'
import type { QuoteStatus } from './types'

const VARIANTS: Record<QuoteStatus, BadgeTone> = {
  draft: 'neutral',
  pending_approval: 'warning',
  approved: 'primary',
  sent: 'primary',
  accepted: 'success',
  rejected: 'destructive',
  expired: 'neutral',
}

export function QuoteStatusBadge({ status }: { status: QuoteStatus }) {
  const { t } = useTranslation('quotes')
  return <Badge tone={VARIANTS[status]}>{t(`status.${status}`)}</Badge>
}
