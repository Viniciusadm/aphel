import { useEntities } from '@/sdk/entities'
import { BellOff, BellRing } from 'lucide-react'
import { createElement, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDateTime, NativeSelect, recordTypeIcon, useRecordFormatting } from '@/sdk'
import { FOLLOWED_PER_PAGE, useFollowedRecords, useFollowToggle } from './api'
import { followedRecordHref, useRecordTypeLabel } from './labels'
import type { FollowedRecord } from './types'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function FollowingPage() {
  const { t } = useTranslation('followers')
  const entities = useEntities()
  const [recordType, setRecordType] = useState('')
  const [page, setPage] = useState(1)
  const followed = useFollowedRecords(recordType, page)
  const typeLabel = useRecordTypeLabel()

  let body
  if (followed.isPending) {
    body = (
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3" aria-busy="true">
        <Skeleton className="h-32 w-full" />
        <Skeleton className="h-32 w-full" />
        <Skeleton className="h-32 w-full" />
      </div>
    )
  } else if (followed.isError) {
    body = <ErrorState onRetry={() => void followed.refetch()} />
  } else if (followed.data.data.length === 0) {
    body = <EmptyState icon={BellRing} title={t('page.emptyTitle')} description={t('page.emptyDescription')} />
  } else {
    const total = followed.data.meta.total
    const pages = Math.max(1, Math.ceil(total / FOLLOWED_PER_PAGE))
    body = (
      <>
        <ul className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {followed.data.data.map((record) => (
            <FollowedCard key={`${record.record_type}:${record.record_id}`} record={record} typeLabel={typeLabel} />
          ))}
        </ul>
        {pages > 1 && (
          <nav
            aria-label={t('pager.label')}
            className="flex flex-wrap items-center justify-between gap-2 text-sm text-muted-foreground"
          >
            <span>{t('pager.summary', { page, pages, total })}</span>
            <div className="flex gap-2">
              <Button variant="outline" size="sm" onClick={() => setPage(page - 1)} disabled={page <= 1}>
                {t('pager.previous')}
              </Button>
              <Button variant="outline" size="sm" onClick={() => setPage(page + 1)} disabled={page >= pages}>
                {t('pager.next')}
              </Button>
            </div>
          </nav>
        )}
      </>
    )
  }

  const total = followed.data?.meta.total

  return (
    <Page>
      <PageHeader title={t('page.title')} description={t('page.description')} />
      <div className="flex flex-wrap items-center justify-between gap-3">
        <NativeSelect
          aria-label={t('page.typeFilter')}
          className="w-56"
          value={recordType}
          onChange={(event) => {
            setRecordType(event.target.value)
            setPage(1)
          }}
        >
          <option value="">{t('page.allTypes')}</option>
          {(entities.data ?? []).filter((entity) => entity.access.read).map((entity) => (
            <option key={entity.record_type} value={entity.record_type}>
              {typeLabel(entity.record_type)}
            </option>
          ))}
        </NativeSelect>
        {total !== undefined && <span className="text-sm text-muted-foreground tabular-nums">{t('page.total', { count: total })}</span>}
      </div>
      {body}
    </Page>
  )
}

interface FollowedCardProps {
  record: FollowedRecord
  typeLabel: (recordType: string) => string
}

function FollowedCard({ record, typeLabel }: FollowedCardProps) {
  const { t } = useTranslation('followers')
  const formatting = useRecordFormatting()
  const unfollow = useFollowToggle(record.record_type, record.record_id)
  const href = followedRecordHref(record.record_type, record.record_id)
  const title = record.title ?? t('page.untitled')
  return (
    <li className="flex min-w-0 flex-col gap-3 rounded-xl border bg-card p-4 transition-colors hover:bg-accent/30">
      <div className="flex min-w-0 items-start gap-3">
        <span className="inline-flex size-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
          {createElement(recordTypeIcon(record.record_type), { className: 'size-5', 'aria-hidden': true })}
        </span>
        <div className="grid min-w-0 flex-1 gap-0.5">
          {href ? (
            <Link to={href} className="truncate text-base font-semibold underline-offset-4 hover:underline">
              {title}
            </Link>
          ) : (
            <span className="truncate text-base font-semibold">{title}</span>
          )}
          <span className="truncate text-sm text-muted-foreground">{typeLabel(record.record_type)}</span>
        </div>
        <Badge tone={record.origin === 'owner' ? 'primary' : 'neutral'} className="shrink-0">
          {t(`origin.${record.origin}`)}
        </Badge>
      </div>
      <div className="mt-auto flex items-center justify-between gap-2">
        <span className="text-xs text-muted-foreground">
          {t('page.since', { date: formatDateTime(record.followed_at, formatting) })}
        </span>
        {record.origin === 'manual' && (
          <Button
            size="sm"
            variant="ghost"
            disabled={unfollow.isPending}
            onClick={() => unfollow.mutate(false)}
          >
            <BellOff aria-hidden />
            {t('button.unfollow')}
          </Button>
        )}
      </div>
    </li>
  )
}
