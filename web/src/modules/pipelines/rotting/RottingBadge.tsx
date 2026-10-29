import { AlarmClock } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'
import { rottingOf } from './lib'

export interface RottingBadgeProps {
  deal: object
  className?: string
}

export function RottingBadge({ deal, className }: RottingBadgeProps) {
  const { t } = useTranslation('pipelines', { keyPrefix: 'rotting' })
  const rotting = rottingOf(deal)
  if (!rotting.isRotting) return null
  const days = rotting.daysInStage ?? 0

  return (
    <Badge
      variant="warning"
      className={cn('tabular-nums', className)}
      title={rotting.limit ? t('badge.title', { limit: rotting.limit }) : undefined}
    >
      <AlarmClock className="size-3.5" aria-hidden="true" />
      {t('badge.daysInStage', { count: days })}
    </Badge>
  )
}
