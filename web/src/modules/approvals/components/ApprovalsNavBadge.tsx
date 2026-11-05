import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'
import {usePendingApprovalsCount} from '@/sdk'

export function ApprovalsNavBadge({ collapsed = false }: { collapsed?: boolean }) {
  const { t } = useTranslation('approvals')
  const count = usePendingApprovalsCount()
  const value = count.data ?? 0
  if (value <= 0) return null
  const label = value > 99 ? '99+' : String(value)
  return (
    <Badge
      count
      size="sm"
      aria-label={t('nav.pending', { count: value })}
      className={cn(collapsed ? 'absolute top-0.5 right-0.5' : 'ml-auto')}
    >
      {label}
    </Badge>
  )
}
