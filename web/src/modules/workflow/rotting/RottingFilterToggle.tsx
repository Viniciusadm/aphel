import { useFieldSchema } from '@/features/records/api'
import { AlarmClock } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import type { SlotProps } from '@/kernel/modules/slots'
import { cn } from '@/lib/utils'
import { hasRottingFilter, withRottingFilter } from './filter'

export function RottingFilterToggle({ recordType, filter, applyFilter }: SlotProps<'record.list.toolbar'>) {
  const { t } = useTranslation('workflow', { keyPrefix: 'rotting' })
  const active = hasRottingFilter(filter)
  const schema = useFieldSchema(recordType)
  if (!schema.data?.fields.some((field) => field.key === 'is_rotting')) return null

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
