import { AlarmClock } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'
import { rottingOf } from './lib'

export interface RottingBadgeProps {
  record: object
  className?: string
}

export function RottingBadge({ record, className }: RottingBadgeProps) {
  const { t } = useTranslation('workflow', { keyPrefix: 'rotting' })
  const rotting = rottingOf(record)
  if (!rotting.isRotting) return null
  const days = rotting.daysInStage ?? 0

  return (
    <Badge
      tone="warning"
      icon={<AlarmClock aria-hidden="true" />}
      className={cn('tabular-nums', className)}
      title={rotting.limit ? t('badge.title', { limit: rotting.limit }) : undefined}
    >
      {t('badge.daysInStage', { count: days })}
    </Badge>
  )
}
