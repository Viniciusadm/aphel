import { PaginationSummary } from '@/components/ui/pagination-summary'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { getRecordResource } from '@/sdk'
import { Pagination } from '@/components/ui/pagination'
import { MEMBERS_PER_PAGE, isSegmentRecordType, useSegment, useSegmentMembers } from './api'
import { SEGMENT_TYPE_PARAM } from './SegmentsPage'
import '@/modules/people/resource'
import '@/modules/organizations/resource'
import '@/modules/deals/resource'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'

export function SegmentDetailPage() {
  const { t } = useTranslation('segments')
  const { id = '' } = useParams()
  const segment = useSegment(id)
  const [page, setPage] = useState(1)
  const members = useSegmentMembers(id, page)

  if (segment.isPending) return <Skeleton className="h-64 w-full" />
  if (segment.isError) return <ErrorState onRetry={() => void segment.refetch()} />

  const resource = getRecordResource(segment.data.record_type)
  const recordType = segment.data.record_type
  const backTo = isSegmentRecordType(recordType) ? `/segments?${SEGMENT_TYPE_PARAM}=${recordType}` : '/segments'

  return (
    <Page>
      <PageHeader
        back={{ to: backTo, label: t('detail.back') }}
        title={segment.data.name}
        description={
          <>
            {segment.data.description && <p>{segment.data.description}</p>}
            <p>
              {isSegmentRecordType(recordType) && t(`recordTypes.${recordType}`)}
              {' · '}
              {members.data ? t('page.count', { count: members.data.meta.total }) : t('detail.loading')}
            </p>
          </>
        }
      />
      {members.isPending ? (
        <Skeleton className="h-40 w-full" />
      ) : members.isError ? (
        <ErrorState description={t('detail.invalid')} onRetry={() => void members.refetch()} />
      ) : members.data.data.length === 0 ? (
        <EmptyState title={t('detail.emptyTitle')} description={t('detail.emptyDescription')} />
      ) : (
        <>
          <PaginationSummary page={page} perPage={MEMBERS_PER_PAGE} total={members.data.meta.total} />
          <ul className="divide-y rounded-lg border" aria-label={t('detail.membersLabel')}>
            {members.data.data.map((member) => {
              const label = member.label ?? member.id
              const href = resource?.detailPath?.(member.id)
              return (
                <li key={member.id} className="min-w-0 px-4 py-2.5 text-sm">
                  {href ? (
                    <Link to={href} className="block truncate font-medium underline-offset-4 hover:underline">
                      {label}
                    </Link>
                  ) : (
                    <span className="block truncate">{label}</span>
                  )}
                </li>
              )
            })}
          </ul>
          <Pagination
            page={page}
            perPage={MEMBERS_PER_PAGE}
            total={members.data.meta.total}
            onPageChange={setPage}
          />
        </>
      )}
    </Page>
  )
}
