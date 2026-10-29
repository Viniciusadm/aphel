import { AlarmClock } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import type { SlotProps } from '@/kernel/modules/slots'
import { cn } from '@/lib/utils'
import { hasRottingFilter, withRottingFilter } from './filter'

export function RottingFilterToggle({ filter, applyFilter }: SlotProps<'record.list.toolbar'>) {
  const { t } = useTranslation('pipelines', { keyPrefix: 'rotting' })
  const active = hasRottingFilter(filter)

  return (
    <Button
      type="button"
      variant="outline"
      size="sm"
      aria-pressed={active}
      title={t('filter.description')}
      className={cn('h-8', active && 'border-warning-solid bg-warning-solid text-warning-foreground')}
      onClick={() => applyFilter(withRottingFilter(filter, !active))}
    >
      <AlarmClock aria-hidden="true" />
      {t('filter.label')}
    </Button>
  )
}
