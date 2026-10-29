import type { ComponentType, ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { formatDayNumber, formatDayTitle, formatWeekdayShort } from './dates'
import type { DayCellProps } from './MonthGrid'

function PlainCell({ className, children }: DayCellProps) {
  return <div className={className}>{children}</div>
}

export interface WeekColumnsProps<T> {
  days: string[]
  todayKey: string
  locale: string
  itemsByDay: Map<string, T[]>
  getKey: (item: T) => string
  renderItem: (item: T) => ReactNode
  Cell?: ComponentType<DayCellProps>
}

export function WeekColumns<T>({
  days,
  todayKey,
  locale,
  itemsByDay,
  getKey,
  renderItem,
  Cell = PlainCell,
}: WeekColumnsProps<T>) {
  const { t } = useTranslation('calendar')
  return (
    <div className="min-w-0 overflow-hidden rounded-md border" role="grid" aria-label={t('grid.week')}>
      <div className="grid grid-cols-7 border-b bg-muted/40" role="row">
        {days.map((key) => (
          <div
            key={key}
            role="columnheader"
            aria-label={formatDayTitle(key, locale)}
            className="flex min-w-0 items-center gap-1 px-2 py-1.5 text-xs font-medium text-muted-foreground capitalize"
          >
            <span className="truncate">{formatWeekdayShort(key, locale)}</span>
            <span
              className={cn(
                'inline-flex size-6 shrink-0 items-center justify-center rounded-full',
                key === todayKey && 'bg-primary-solid font-semibold text-primary-foreground',
              )}
            >
              {formatDayNumber(key)}
            </span>
          </div>
        ))}
      </div>
      <div className="grid grid-cols-7" role="row">
        {days.map((key, index) => (
          <Cell
            key={key}
            dayKey={key}
            className={cn('flex min-h-96 min-w-0 flex-col gap-1 p-1', index % 7 !== 6 && 'border-r')}
          >
            <ul className="grid min-w-0 gap-0.5" role="gridcell">
              {(itemsByDay.get(key) ?? []).map((item) => (
                <li key={getKey(item)} className="min-w-0">
                  {renderItem(item)}
                </li>
              ))}
            </ul>
          </Cell>
        ))}
      </div>
    </div>
  )
}
