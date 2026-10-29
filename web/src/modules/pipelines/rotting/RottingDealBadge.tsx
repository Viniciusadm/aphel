import { AlarmClock } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import type { SlotProps } from '@/kernel/modules/slots'
import { rottingOf } from './lib'
import { RottingBadge } from './RottingBadge'

export function RottingDealBadge({ deal, placement }: SlotProps<'deals.badges'>) {
  const { t } = useTranslation('pipelines', { keyPrefix: 'rotting' })
  if (placement === 'list') return <RottingBadge deal={deal} />
  const rotting = rottingOf(deal)
  if (!rotting.isRotting) return null

  return (
    <Badge variant="warning" title={rotting.limit ? t('badge.title', { limit: rotting.limit }) : undefined}>
      <AlarmClock className="size-3.5" aria-hidden="true" />
      {t('filter.label')}
    </Badge>
  )
}
