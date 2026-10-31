import { useTranslation } from 'react-i18next'
import { Badge, type BadgeTone } from '@/components/ui/badge'

const VARIANTS: Record<string, BadgeTone> = {
  pending: 'warning',
  queued: 'warning',
  processing: 'primary',
  running: 'primary',
  completed: 'success',
  failed: 'destructive',
  cancelled: 'neutral',
}

export function StatusBadge({ status }: { status: string }) {
  const { t } = useTranslation('privacy')
  return <Badge tone={VARIANTS[status] ?? 'neutral'}>{t(`status.${status as 'pending'}`)}</Badge>
}
