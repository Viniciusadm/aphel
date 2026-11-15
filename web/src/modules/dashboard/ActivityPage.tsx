import { Activity } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Skeleton } from '@/components/ui/skeleton'
import { entityLabel, useEntities } from '@/sdk/entities'
import { NativeSelect, useOrganizationUsers } from '@/sdk'
import { useActivity } from './api'
import { ActivityEventRow } from './widgets/RecentActivityWidget'

const PERIODS = [1, 7, 30, 90]

export function ActivityPage() {
  const { t, i18n } = useTranslation('dashboard')
  const entities = useEntities()
  const users = useOrganizationUsers()
  const [days, setDays] = useState(7)
  const [recordType, setRecordType] = useState('')
  const [actorId, setActorId] = useState('')
  const activity = useActivity({ days, recordType, actorId })
  const items = activity.data?.pages.flatMap((page) => page.data) ?? []
  const filtered = recordType !== '' || actorId !== ''

  let body
  if (activity.isPending) {
    body = <Skeleton className="h-48 w-full" aria-busy="true" />
  } else if (activity.isError) {
    body = <ErrorState onRetry={() => void activity.refetch()} />
  } else if (items.length === 0) {
    body = (
      <EmptyState
        icon={Activity}
        title={t(filtered ? 'activity.emptyFilteredTitle' : 'activity.emptyTitle')}
        description={t(filtered ? 'activity.emptyFilteredDescription' : 'activity.emptyDescription')}
      />
    )
  } else {
    body = (
      <>
        <ol className="grid gap-4 rounded-lg border bg-card p-4" aria-label={t('activity.title')}>
          {items.map((event) => (
            <ActivityEventRow key={event.id} event={event} />
          ))}
        </ol>
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
      <div className="flex flex-wrap gap-3">
        <NativeSelect aria-label={t('activity.period')} className="w-full sm:w-48" value={String(days)} onChange={(event) => setDays(Number(event.target.value))}>
          {PERIODS.map((value) => (
            <option key={value} value={value}>
              {t(`activity.periods.d${value}`)}
            </option>
          ))}
        </NativeSelect>
        <NativeSelect aria-label={t('activity.recordType')} className="w-full sm:w-48" value={recordType} onChange={(event) => setRecordType(event.target.value)}>
          <option value="">{t('activity.allTypes')}</option>
          {(entities.data ?? [])
            .filter((entity) => entity.access.read && 'ui.plural_labels' in entity.traits)
            .map((entity) => (
              <option key={entity.record_type} value={entity.record_type}>
                {entityLabel(entity, i18n.language, entity.record_type)}
              </option>
            ))}
        </NativeSelect>
        <NativeSelect aria-label={t('activity.person')} className="w-full sm:w-56" value={actorId} onChange={(event) => setActorId(event.target.value)}>
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
