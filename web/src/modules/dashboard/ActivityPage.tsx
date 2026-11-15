import { Activity } from 'lucide-react'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'
import { utcToZonedInput } from '@/components/fields/lib/datetime'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { Skeleton } from '@/components/ui/skeleton'
import { entityLabel, useEntities } from '@/sdk/entities'
import { NativeSelect, useOrganizationUsers, useRecordFormatting } from '@/sdk'
import { useActivity } from './api'
import { ActivityEventRow } from './widgets/RecentActivityWidget'

const PERIODS = ['1', '7', '30', '90'] as const
type Period = (typeof PERIODS)[number]

function dayKey(value: string, timeZone: string) {
  return utcToZonedInput(value, timeZone).slice(0, 10)
}

export function ActivityPage() {
  const { t, i18n } = useTranslation('dashboard')
  const formatting = useRecordFormatting()
  const entities = useEntities()
  const users = useOrganizationUsers()
  const [params, setParams] = useSearchParams()
  const period: Period = PERIODS.includes(params.get('period') as Period) ? (params.get('period') as Period) : '7'
  const recordType = params.get('type') ?? ''
  const actorId = params.get('person') ?? ''
  const activity = useActivity({ days: Number(period), recordType, actorId })
  const items = useMemo(() => activity.data?.pages.flatMap((page) => page.data) ?? [], [activity.data])
  const filtered = recordType !== '' || actorId !== ''

  const update = (key: string, value: string) =>
    setParams(
      (previous) => {
        const next = new URLSearchParams(previous)
        if (value === '' || (key === 'period' && value === '7')) next.delete(key)
        else next.set(key, value)
        return next
      },
      { replace: true },
    )

  const groups = useMemo(() => {
    const today = dayKey(new Date().toISOString(), formatting.timezone)
    const yesterday = dayKey(new Date(Date.now() - 86_400_000).toISOString(), formatting.timezone)
    const map = new Map<string, typeof items>()
    for (const event of items) {
      const key = dayKey(event.occurred_at, formatting.timezone)
      map.set(key, [...(map.get(key) ?? []), event])
    }
    return [...map.entries()].map(([key, events]) => ({
      key,
      events,
      label:
        key === today
          ? t('activity.today')
          : key === yesterday
            ? t('activity.yesterday')
            : new Intl.DateTimeFormat(formatting.locale, { weekday: 'long', day: 'numeric', month: 'long', timeZone: formatting.timezone }).format(new Date(events[0].occurred_at)),
    }))
  }, [items, formatting.timezone, formatting.locale, t])

  let body
  if (activity.isPending) {
    body = (
      <div className="grid gap-3" aria-busy="true">
        <Skeleton className="h-6 w-32" />
        <Skeleton className="h-40 w-full" />
      </div>
    )
  } else if (activity.isError) {
    body = <ErrorState onRetry={() => void activity.refetch()} />
  } else if (items.length === 0) {
    body = (
      <EmptyState
        icon={Activity}
        title={t(filtered ? 'activity.emptyFilteredTitle' : 'activity.emptyTitle')}
        description={t(filtered ? 'activity.emptyFilteredDescription' : 'activity.emptyDescription')}
        action={
          filtered ? (
            <Button size="sm" variant="outline" onClick={() => setParams({}, { replace: true })}>
              {t('activity.clearFilters')}
            </Button>
          ) : (
            <Button size="sm" asChild>
              <Link to="/following">{t('activity.seeFollowed')}</Link>
            </Button>
          )
        }
      />
    )
  } else {
    body = (
      <>
        {groups.map((group) => (
          <section key={group.key} className="grid gap-2" aria-label={group.label}>
            <h2 className="text-sm font-semibold capitalize text-muted-foreground">{group.label}</h2>
            <ol className="grid gap-4 rounded-lg border bg-card p-4">
              {group.events.map((event) => (
                <ActivityEventRow key={event.id} event={event} timeOnly />
              ))}
            </ol>
          </section>
        ))}
        {activity.hasNextPage && (
          <div className="flex justify-center">
            <Button variant="outline" disabled={activity.isFetchingNextPage} onClick={() => void activity.fetchNextPage()}>
              {activity.isFetchingNextPage ? t('activity.loading') : t('activity.loadMore')}
            </Button>
          </div>
        )}
      </>
    )
  }

  return (
    <Page>
      <PageHeader title={t('activity.title')} description={t('activity.description')} />
      <div className="flex flex-wrap items-center gap-3">
        <SegmentedControl
          aria-label={t('activity.period')}
          value={period}
          onValueChange={(value) => update('period', value)}
          options={PERIODS.map((value) => ({ value, label: t(`activity.periods.d${value}`) }))}
        />
        <NativeSelect aria-label={t('activity.recordType')} className="w-full sm:w-48" value={recordType} onChange={(event) => update('type', event.target.value)}>
          <option value="">{t('activity.allTypes')}</option>
          {(entities.data ?? [])
            .filter((entity) => entity.access.read && 'ui.plural_labels' in entity.traits)
            .map((entity) => (
              <option key={entity.record_type} value={entity.record_type}>
                {entityLabel(entity, i18n.language, entity.record_type)}
              </option>
            ))}
        </NativeSelect>
        <NativeSelect aria-label={t('activity.person')} className="w-full sm:w-56" value={actorId} onChange={(event) => update('person', event.target.value)}>
          <option value="">{t('activity.everyone')}</option>
          {(users.data ?? []).map((user) => (
            <option key={user.id} value={user.id}>
              {user.display_name}
            </option>
          ))}
        </NativeSelect>
      </div>
      {body}
    </Page>
  )
}
