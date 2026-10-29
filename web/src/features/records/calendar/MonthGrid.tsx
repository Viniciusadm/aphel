import { useState, type ComponentType, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { formatDayNumber, formatDayTitle, formatWeekdayShort, monthGrid, sameMonth } from './dates'

export interface DayCellProps {
  dayKey: string
  className: string
  children: ReactNode
}

function PlainCell({ className, children }: DayCellProps) {
  return <div className={className}>{children}</div>
}

export interface MonthGridProps<T> {
  monthKey: string
  todayKey: string
  locale: string
  itemsByDay: Map<string, T[]>
  getKey: (item: T) => string
  renderItem: (item: T) => ReactNode
  maxVisible?: number
  onDayClick?: (dayKey: string) => void
  onShowMore?: (dayKey: string) => void
  dayAction?: (dayKey: string) => ReactNode
  Cell?: ComponentType<DayCellProps>
  weekStartsOn?: number
}

export function MonthGrid<T>({
  monthKey,
  todayKey,
  locale,
  itemsByDay,
  getKey,
  renderItem,
  maxVisible = 3,
  onDayClick,
  onShowMore,
  dayAction,
  Cell = PlainCell,
  weekStartsOn,
}: MonthGridProps<T>) {
  const { t } = useTranslation('calendar')
  const [expanded, setExpanded] = useState<string | null>(null)
  const days = monthGrid(monthKey, weekStartsOn)

  return (
    <div className="min-w-0 overflow-hidden rounded-md border" role="grid" aria-label={t('grid.month')}>
      <div className="grid grid-cols-7 border-b bg-muted/40" role="row">
        {days.slice(0, 7).map((key) => (
          <div
            key={key}
            role="columnheader"
            className="truncate px-2 py-1.5 text-xs font-medium text-muted-foreground capitalize"
          >
            {formatWeekdayShort(key, locale)}
          </div>
        ))}
      </div>
      <div className="grid grid-cols-7">
        {days.map((key, index) => {
          const items = itemsByDay.get(key) ?? []
          const showAll = expanded === key
          const visible = showAll ? items : items.slice(0, maxVisible)
          const hidden = items.length - visible.length
          const isToday = key === todayKey
          return (
            <Cell
              key={key}
              dayKey={key}
              className={cn(
                'group/day relative flex min-h-28 min-w-0 flex-col gap-1 border-b p-1',
                index % 7 !== 6 && 'border-r',
                index >= 35 && 'border-b-0',
                !sameMonth(key, monthKey) && 'bg-muted/30 text-muted-foreground',
              )}
            >
              <div className="flex items-center justify-between gap-1" role="gridcell">
                {onDayClick ? (
                  <button
                    type="button"
                    onClick={() => onDayClick(key)}
                    aria-label={formatDayTitle(key, locale)}
                    className={cn(
                      'inline-flex size-6 items-center justify-center rounded-full text-xs hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none',
                      isToday && 'bg-primary-solid font-semibold text-primary-foreground hover:bg-primary-solid/90',
                    )}
                  >
                    {formatDayNumber(key)}
                  </button>
                ) : (
                  <span
                    aria-label={formatDayTitle(key, locale)}
                    className={cn(
                      'inline-flex size-6 items-center justify-center rounded-full text-xs',
                      isToday && 'bg-primary-solid font-semibold text-primary-foreground',
                    )}
                  >
                    {formatDayNumber(key)}
                  </span>
                )}
                {dayAction?.(key)}
              </div>
              <ul className="grid min-w-0 gap-0.5">
                {visible.map((item) => (
                  <li key={getKey(item)} className="min-w-0">
                    {renderItem(item)}
                  </li>
                ))}
              </ul>
              {hidden > 0 && (
                <button
                  type="button"
                  className="self-start rounded px-1 text-xs font-medium text-muted-foreground hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
                  onClick={() => (onShowMore ? onShowMore(key) : setExpanded(key))}
                >
                  {t('grid.more', { count: hidden })}
                </button>
              )}
              {showAll && items.length > maxVisible && !onShowMore && (
                <button
                  type="button"
                  className="self-start rounded px-1 text-xs text-muted-foreground hover:text-foreground"
                  onClick={() => setExpanded(null)}
                >
                  {t('grid.less')}
                </button>
              )}
            </Cell>
          )
        })}
      </div>
    </div>
  )
}
