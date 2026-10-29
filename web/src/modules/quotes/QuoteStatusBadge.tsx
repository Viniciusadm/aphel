import { useTranslation } from 'react-i18next'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import type { QuoteStatus } from './types'

const VARIANTS: Record<QuoteStatus, BadgeProps['variant']> = {
  draft: 'secondary',
  pending_approval: 'warning',
  approved: 'default',
  sent: 'outline',
  accepted: 'success',
  rejected: 'destructive',
  expired: 'secondary',
}

export function QuoteStatusBadge({ status }: { status: QuoteStatus }) {
  const { t } = useTranslation('quotes')
  return <Badge variant={VARIANTS[status]}>{t(`status.${status}`)}</Badge>
}
