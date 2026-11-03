import { AlarmClock } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { stageRottingDays } from './lib'

export function RottingStageBadge({ stage }: { stage: object }) {
  const { t } = useTranslation('workflow', { keyPrefix: 'rotting' })
  const days = stageRottingDays(stage)
  if (days === null) return null

  return (
    <Badge variant="outline">
      <AlarmClock className="size-3.5" aria-hidden="true" />
      {t('stage.summary', { count: days })}
    </Badge>
  )
}
