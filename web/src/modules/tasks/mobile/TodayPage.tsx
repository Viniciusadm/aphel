import { CalendarDays, CloudOff, ListChecks, PartyPopper } from 'lucide-react'
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { notifyError } from '@/components/feedback/notify'
import { Badge } from '@/components/ui/badge'
import { Checkbox } from '@/components/ui/checkbox'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useToggleActivity, type Activity, type InboxItem } from '../api'
import { ActivityTypeIcon } from '../components/ActivityTypeIcon'
import { activityResource } from '../resource'
import { eventStart } from '@/sdk/calendar/events'
import { formatDayTitle, formatMinutes, todayKey } from '@/sdk/calendar/dates'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { getRecordResource } from '@/features/records/resources'
import { cn } from '@/lib/utils'
import { useOfflineQuery } from '@/pwa/offline/cache'
import { TODAY_TASK_GROUPS, todayQueries, useTodayRange } from './api'
import { ActivityMapLink } from '@/features/mobile/components/ActivityMapLink'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

function RecordReference({ activity }: { activity: Activity }) {
  if (!activity.record) return null
  const href = getRecordResource(activity.record.record_type)?.detailPath?.(activity.record.record_id)
  const label = activity.record.title ?? ''
  if (!label) return null
  return href ? (
    <Link to={href} className="truncate text-xs text-primary underline-offset-4 hover:underline">
      {label}
    </Link>
  ) : (
    <span className="truncate text-xs text-muted-foreground">{label}</span>
  )
}

function ActivityTitle({ activity, finished }: { activity: Activity; finished: boolean }) {
  const href = activityResource.detailPath?.(activity.id)
  const className = cn('min-w-0 truncate text-sm font-medium', finished && 'text-muted-foreground line-through')
  return href ? (
    <Link to={href} className={cn(className, 'hover:underline')}>
      {activity.title}
    </Link>
  ) : (
    <span className={className}>{activity.title}</span>
  )
}

function CompleteToggle({ activity }: { activity: Activity }) {
  const { t } = useTranslation('tasks')
  const toggle = useToggleActivity()
  const done = activity.status === 'done'
  if (activity.status === 'canceled') return null
  return (
    <label className="-m-2 flex size-11 shrink-0 cursor-pointer items-center justify-center">
      <Checkbox
        checked={done}
        disabled={toggle.isPending}
        aria-label={t(done ? 'today.reopen' : 'today.complete', { title: activity.title })}
        onCheckedChange={() =>
          toggle.mutate({ id: activity.id, done: !done }, { onError: (error) => notifyError(error) })
        }
      />
    </label>
  )
}

function AgendaRow({ activity }: { activity: Activity }) {
  const { timezone, locale } = useRecordFormatting()
  const finished = activity.status === 'done' || activity.status === 'canceled'
  return (
    <li className="flex items-start gap-3 py-3">
      <span className="w-14 shrink-0 pt-1 text-xs font-medium text-muted-foreground tabular-nums">
        {activity.due_at ? formatMinutes(eventStart(activity, timezone), locale) : ''}
      </span>
      <ActivityTypeIcon icon={activity.activity_type.icon} color={activity.activity_type.color} />
      <div className="grid min-w-0 flex-1 gap-0.5">
        <ActivityTitle activity={activity} finished={finished} />
        <RecordReference activity={activity} />
        <ActivityMapLink location={activity.location} />
      </div>
      <CompleteToggle activity={activity} />
    </li>
  )
}

function TaskRow({ item, overdue }: { item: InboxItem; overdue: boolean }) {
  const { t } = useTranslation('tasks')
  const formatting = useRecordFormatting()
  return (
    <li className="flex items-start gap-3 py-3">
      <CompleteToggle activity={item} />
      <div className="grid min-w-0 flex-1 gap-0.5">
        <ActivityTitle activity={item} finished={false} />
        {item.due_at && (
          <span className={cn('text-xs', overdue ? 'text-destructive' : 'text-muted-foreground')}>
            {t(overdue ? 'today.overdueSince' : 'today.dueAt', { date: formatDateTime(item.due_at, formatting) })}
          </span>
        )}
        <RecordReference activity={item} />
      </div>
      {overdue && <Badge variant="destructive">{t('today.overdue')}</Badge>}
    </li>
  )
}

function SectionSkeleton() {
  return (
    <div className="grid gap-3 py-2" aria-busy="true">
      <Skeleton className="h-10 w-full" />
      <Skeleton className="h-10 w-full" />
      <Skeleton className="h-10 w-full" />
    </div>
  )
}

function Section({ icon: Icon, title, count, children }: { icon: typeof CalendarDays; title: string; count?: number; children: ReactNode }) {
  return (
    <section className="grid gap-1 rounded-xl border bg-card px-4 pt-3 pb-1">
      <h2 className="flex items-center gap-2 text-sm font-semibold">
        <Icon className="size-4 text-muted-foreground" aria-hidden="true" />
        {title}
        {count !== undefined && <span className="text-muted-foreground">({count})</span>}
      </h2>
      {children}
    </section>
  )
}

export function TodayPage() {
  const { t } = useTranslation('tasks')
  const formatting = useRecordFormatting()
  const range = useTodayRange()
  const agenda = useOfflineQuery(todayQueries.agenda(range))
  const tasks = useOfflineQuery(todayQueries.tasks())
  const agendaItems = [...(agenda.query.data?.items ?? [])].sort((left, right) =>
    (left.due_at ?? '').localeCompare(right.due_at ?? ''),
  )
  const taskGroups = (tasks.query.data?.groups ?? []).filter((group) =>
    (TODAY_TASK_GROUPS as readonly string[]).includes(group.key),
  )
  const taskCount = taskGroups.reduce((total, group) => total + group.items.length, 0)
  const fromCache = agenda.fromCache || tasks.fromCache

  return (
    <Page className="mx-auto max-w-2xl">
<PageHeader
        title={t('today.title')}
        description={<p className="first-letter:uppercase">{formatDayTitle(todayKey(formatting.timezone), formatting.locale)}</p>}
      />
      {fromCache && (
        <p className="flex items-center gap-2 rounded-md border border-dashed px-3 py-2 text-xs text-muted-foreground" role="status">
          <CloudOff className="size-4 shrink-0" aria-hidden="true" />
          {t('today.fromCache')}
        </p>
      )}
      <Section icon={CalendarDays} title={t('today.agenda')} count={agenda.query.data ? agendaItems.length : undefined}>
        {agenda.query.isPending ? (
          <SectionSkeleton />
        ) : agenda.query.isError && !agenda.query.data ? (
          <ErrorState onRetry={() => void agenda.query.refetch()} />
        ) : agendaItems.length === 0 ? (
          <p className="py-4 text-sm text-muted-foreground">{t('today.agendaEmpty')}</p>
        ) : (
          <ul className="divide-y">
            {agendaItems.map((activity) => (
              <AgendaRow key={activity.id} activity={activity} />
            ))}
          </ul>
        )}
      </Section>
      <Section icon={ListChecks} title={t('today.tasks')} count={tasks.query.data ? taskCount : undefined}>
        {tasks.query.isPending ? (
          <SectionSkeleton />
        ) : tasks.query.isError && !tasks.query.data ? (
          <ErrorState onRetry={() => void tasks.query.refetch()} />
        ) : taskCount === 0 ? (
          <EmptyState icon={PartyPopper} title={t('today.tasksEmpty')} description={t('today.tasksEmptyDescription')} />
        ) : (
          <ul className="divide-y">
            {taskGroups.flatMap((group) =>
              group.items.map((item) => <TaskRow key={item.id} item={item} overdue={group.key === 'overdue'} />),
            )}
          </ul>
        )}
      </Section>
      {tasks.query.data?.truncated && (
        <Link to="/inbox" className="text-center text-sm text-primary underline-offset-4 hover:underline">
          {t('today.seeInbox')}
        </Link>
      )}
    </Page>
  )
}
