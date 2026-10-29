import { AlarmClock, CalendarClock, CalendarDays, Hourglass, type LucideIcon } from 'lucide-react'
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { formatCurrency, formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { cn } from '@/lib/utils'
import type { Bucket, PendingActivityItem, PendingCenterData, StaleDealItem } from '../types'
import { RecordLinkText } from '@/kernel/widgets/shared'

interface SectionProps<T> {
  id: string
  icon: LucideIcon
  title: string
  bucket: Bucket<T>
  tone?: 'danger' | 'default'
  renderItem: (item: T) => ReactNode
}

function Section<T extends { id: string }>({ id, icon: Icon, title, bucket, tone = 'default', renderItem }: SectionProps<T>) {
  const { t } = useTranslation('dashboard')
  const headingId = `${id}-heading`
  const remaining = bucket.count - bucket.items.length
  return (
    <section aria-labelledby={headingId} className="flex min-w-0 flex-col gap-2">
      <h4 id={headingId} className="flex items-center gap-2 text-sm font-medium">
        <Icon className={cn('size-4', tone === 'danger' && bucket.count > 0 ? 'text-destructive' : 'text-muted-foreground')} aria-hidden />
        <span className="truncate">{title}</span>
        <span
          className={cn(
            'ml-auto rounded-full px-2 py-0.5 text-xs tabular-nums',
            tone === 'danger' && bucket.count > 0 ? 'bg-destructive/10 text-destructive' : 'bg-muted text-muted-foreground',
          )}
        >
          {bucket.count}
        </span>
      </h4>
      {bucket.count === 0 ? (
        <p className="text-xs text-muted-foreground">{t('widgets.pending_center.noItems')}</p>
      ) : (
        <ul className="grid gap-1.5">
          {bucket.items.map((item) => (
            <li key={item.id} className="min-w-0 rounded-md border px-3 py-2 text-sm">
              {renderItem(item)}
            </li>
          ))}
          {remaining > 0 && (
            <li className="text-xs text-muted-foreground">{t('widgets.pending_center.more', { count: remaining })}</li>
          )}
        </ul>
      )}
    </section>
  )
}

export function PendingCenterWidget({ data }: { data: PendingCenterData }) {
  const { t } = useTranslation('dashboard')
  const formatting = useRecordFormatting()
  const activity = (item: PendingActivityItem) => (
    <div className="grid min-w-0 gap-0.5">
      <Link to={`/tasks/${encodeURIComponent(item.id)}`} className="truncate font-medium hover:underline">
        {item.title}
      </Link>
      <p className="truncate text-xs text-muted-foreground">
        {item.due_at && t('widgets.pending_center.dueAt', { date: formatDateTime(item.due_at, formatting) })}
        {item.record && (
          <>
            {' · '}
            <RecordLinkText record={item.record} fallback={item.record.record_type} />
          </>
        )}
      </p>
    </div>
  )
  const deal = (item: StaleDealItem) => (
    <div className="grid min-w-0 gap-0.5">
      <Link to={`/deals/${encodeURIComponent(item.id)}`} className="truncate font-medium hover:underline">
        {item.title}
      </Link>
      <p className="truncate text-xs text-muted-foreground">
        {item.stage_name}
        {item.amount !== null && ` · ${formatCurrency(item.amount, formatting, item.currency)}`}
        {' · '}
        {t('widgets.pending_center.lastTouched', { date: formatDateTime(item.last_touched_at, formatting) })}
      </p>
    </div>
  )
  return (
    <div className="grid gap-4 sm:grid-cols-2">
      <Section id="pending-overdue" icon={AlarmClock} tone="danger" title={t('widgets.pending_center.overdue')} bucket={data.overdue} renderItem={activity} />
      <Section id="pending-today" icon={CalendarClock} title={t('widgets.pending_center.dueToday')} bucket={data.due_today} renderItem={activity} />
      <Section id="pending-tomorrow" icon={CalendarDays} title={t('widgets.pending_center.dueTomorrow')} bucket={data.due_tomorrow} renderItem={activity} />
      {data.stale_deals ? (
        <Section
          id="pending-stale"
          icon={Hourglass}
          title={t('widgets.pending_center.staleDeals', { count: data.stale_deals.stale_days })}
          bucket={data.stale_deals}
          renderItem={deal}
        />
      ) : null}
    </div>
  )
}
