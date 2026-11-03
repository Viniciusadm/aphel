import { useEntities } from '@/sdk/entities'
import { BellRing } from 'lucide-react'
import { createElement, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { recordTypeIcon } from '@/features/records/links'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { FOLLOWED_PER_PAGE, useFollowedRecords } from './api'
import { followedRecordHref, useRecordTypeLabel } from './labels'
import type { FollowedRecord } from './types'
import { PageHeader } from '@/components/ui/page-header'
import { Field } from '@/components/ui/field'
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
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-14 w-full" />
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
        <ul className="grid gap-2">
          {followed.data.data.map((record) => (
            <FollowedRow key={`${record.record_type}:${record.record_id}`} record={record} typeLabel={typeLabel} />
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

  return (
    <Page>
<PageHeader title={t('page.title')} actions={<>
        <Field label={t('page.typeFilter')}>
          {(control) => <NativeSelect
            {...control}
            className="w-48"
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
          </NativeSelect>}
        </Field>
      
      </>} />
      {body}
    </Page>
  )
}

interface FollowedRowProps {
  record: FollowedRecord
  typeLabel: (recordType: string) => string
}

function FollowedRow({ record, typeLabel }: FollowedRowProps) {
  const { t } = useTranslation('followers')
  const formatting = useRecordFormatting()
  const href = followedRecordHref(record.record_type, record.record_id)
  const title = record.title ?? t('page.untitled')
  return (
    <li className="flex items-center gap-3 rounded-md border px-3 py-2">
      {createElement(recordTypeIcon(record.record_type), {
        className: 'size-4 shrink-0 text-muted-foreground',
        'aria-hidden': true,
      })}
      <div className="grid min-w-0 flex-1">
        {href ? (
          <Link to={href} className="truncate text-sm font-medium hover:underline">
            {title}
          </Link>
        ) : (
          <span className="truncate text-sm font-medium">{title}</span>
        )}
        <span className="truncate text-xs text-muted-foreground">
          {typeLabel(record.record_type)} · {t(`origin.${record.origin}`)}
        </span>
      </div>
      <span className="shrink-0 text-xs text-muted-foreground">
        {t('page.since', { date: formatDateTime(record.followed_at, formatting) })}
      </span>
    </li>
  )
}
