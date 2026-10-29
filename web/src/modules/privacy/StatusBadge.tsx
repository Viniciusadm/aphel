import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'

type BadgeVariant = 'default' | 'secondary' | 'destructive' | 'success' | 'warning' | 'outline'

const VARIANTS: Record<string, BadgeVariant> = {
  pending: 'warning',
  queued: 'warning',
  processing: 'secondary',
  running: 'secondary',
  completed: 'success',
  failed: 'destructive',
  cancelled: 'outline',
}

export function StatusBadge({ status }: { status: string }) {
  const { t } = useTranslation('privacy')
  return <Badge variant={VARIANTS[status] ?? 'outline'}>{t(`status.${status as 'pending'}`)}</Badge>
}
