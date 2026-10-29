import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { formatDayTitle } from './dates'

export interface AgendaListProps<T> {
  days: string[]
  todayKey: string
  locale: string
  itemsByDay: Map<string, T[]>
  getKey: (item: T) => string
  renderItem: (item: T) => ReactNode
  dayAction?: (dayKey: string) => ReactNode
  emptyLabel?: string
}

export function AgendaList<T>({
  days,
  todayKey,
  locale,
  itemsByDay,
  getKey,
  renderItem,
  dayAction,
  emptyLabel,
}: AgendaListProps<T>) {
  const { t } = useTranslation('calendar')
  const withItems = days.filter((key) => (itemsByDay.get(key)?.length ?? 0) > 0)
  if (withItems.length === 0) {
    return <p className="rounded-md border px-3 py-6 text-center text-sm text-muted-foreground">{emptyLabel ?? t('grid.empty')}</p>
  }
  return (
    <div className="grid gap-4">
      {withItems.map((key) => (
        <section key={key} aria-labelledby={`agenda-${key}`} className="grid gap-2">
          <div className="flex items-center justify-between gap-2">
            <h3
              id={`agenda-${key}`}
              className={cn('text-sm font-semibold capitalize', key === todayKey ? 'text-primary' : 'text-muted-foreground')}
            >
              {formatDayTitle(key, locale)}
              {key === todayKey && <span className="ml-2 font-normal normal-case">({t('nav.today')})</span>}
            </h3>
            {dayAction?.(key)}
          </div>
          <ul className="grid gap-1.5">
            {(itemsByDay.get(key) ?? []).map((item) => (
              <li key={getKey(item)} className="min-w-0">
                {renderItem(item)}
              </li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  )
}
