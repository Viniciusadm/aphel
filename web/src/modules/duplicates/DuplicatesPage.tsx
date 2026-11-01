import { PaginationSummary } from '@/components/ui/pagination-summary'
import { ArrowLeftRight, CopyCheck, RefreshCw } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { ListSkeleton } from '@/features/settings/components/ListSkeleton'
import { Pagination } from '@/components/ui/pagination'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import {
  CANDIDATE_STATUSES,
  DUPLICATES_PER_PAGE,
  isCandidateStatus,
  useDismissDuplicate,
  useDuplicateCandidates,
  useDuplicateTypes,
  useRequestDuplicateScan,
  type DuplicateCandidate,
  type DuplicateRecordType,
  type DuplicateTypeInfo,
} from './api'
import { DuplicateRulesCard } from './DuplicateRulesCard'
import { mergePagePath } from './paths'
import { ruleLabelKey, typeLabel } from './lib/labels'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function DuplicatesPage() {
  const { t } = useTranslation('duplicates')
  const types = useDuplicateTypes()

  if (types.isPending) return <ListSkeleton />
  if (types.isError) return <ErrorState onRetry={() => void types.refetch()} />
  if (types.data.length === 0) return <EmptyState icon={CopyCheck} title={t('queue.noTypes')} />
  return <DuplicatesQueue types={types.data} />
}

function DuplicatesQueue({ types }: { types: DuplicateTypeInfo[] }) {
  const { t } = useTranslation('duplicates')
  const { hasPermission } = useSession()
  const [searchParams, setSearchParams] = useSearchParams()
  const typeParam = searchParams.get('type')
  const statusParam = searchParams.get('status')
  const selected = types.find((type) => type.record_type === typeParam) ?? types[0]!
  const recordType: DuplicateRecordType = selected.record_type
  const status = isCandidateStatus(statusParam) ? statusParam : 'pending'
  const page = Math.max(1, Number(searchParams.get('page')) || 1)
  const candidates = useDuplicateCandidates(recordType, status, page)
  const scan = useRequestDuplicateScan()
  const canManage = hasPermission(SETTINGS_MANAGE)

  const update = (next: { type?: DuplicateRecordType; status?: string; page?: number }) => {
    const params = new URLSearchParams()
    params.set('type', next.type ?? recordType)
    params.set('status', next.status ?? status)
    const nextPage = next.page ?? 1
    if (nextPage > 1) params.set('page', String(nextPage))
    setSearchParams(params)
  }

  const requestScan = () =>
    scan.mutate(undefined, {
      onSuccess: () => notifySuccess(t('queue.scanQueued')),
      onError: (error) => notifyError(error),
    })

  return (
    <div className="grid gap-6">
      <Page>
        <PageHeader
          title={t('queue.title')}
          description={t('queue.description')}
          actions={
            canManage && (
              <Button size="sm" variant="outline" disabled={scan.isPending} onClick={requestScan}>
                <RefreshCw aria-hidden="true" />
                {t('queue.scan')}
              </Button>
            )
          }
        />
        <div className="flex flex-wrap items-end gap-3">
          <label className="grid gap-1 text-sm">
            <span className="text-muted-foreground">{t('queue.typeLabel')}</span>
            <NativeSelect
              value={recordType}
              onChange={(event) => update({ type: event.target.value as DuplicateRecordType })}
            >
              {types.map((type) => (
                <option key={type.record_type} value={type.record_type}>
                  {typeLabel(t, type)}
                </option>
              ))}
            </NativeSelect>
          </label>
          <label className="grid gap-1 text-sm">
            <span className="text-muted-foreground">{t('queue.statusLabel')}</span>
            <NativeSelect value={status} onChange={(event) => update({ status: event.target.value })}>
              {CANDIDATE_STATUSES.map((value) => (
                <option key={value} value={value}>
                  {t(`queue.status.${value}`)}
                </option>
              ))}
            </NativeSelect>
          </label>
        </div>
        {candidates.isPending ? (
          <ListSkeleton />
        ) : candidates.isError ? (
          <ErrorState onRetry={() => void candidates.refetch()} />
        ) : candidates.data.data.length === 0 ? (
          <EmptyState
            icon={CopyCheck}
            title={status === 'pending' ? t('queue.emptyTitle') : t('queue.emptyFiltered')}
            description={status === 'pending' ? t('queue.emptyDescription') : undefined}
          />
        ) : (
          <>
            <PaginationSummary page={page} perPage={DUPLICATES_PER_PAGE} total={candidates.data.meta.total} />
            <ul className="grid gap-3" aria-label={t('queue.title')}>
              {candidates.data.data.map((candidate) => (
                <CandidateItem key={candidate.id} candidate={candidate} type={selected} />
              ))}
            </ul>
            <Pagination
              page={page}
              perPage={DUPLICATES_PER_PAGE}
              total={candidates.data.meta.total}
              onPageChange={(next) => update({ page: next })}
            />
          </>
        )}
      </Page>
      {canManage && <DuplicateRulesCard key={recordType} recordType={recordType} label={typeLabel(t, selected)} />}
    </div>
  )
}

function CandidateItem({ candidate, type }: { candidate: DuplicateCandidate; type: DuplicateTypeInfo }) {
  const { t } = useTranslation('duplicates')
  const { hasPermission } = useSession()
  const formatting = useRecordFormatting()
  const dismiss = useDismissDuplicate()
  const recordType = type.record_type
  const permissions = type.permissions
  const pending = candidate.status === 'pending'
  const canMerge = pending && hasPermission(permissions.update) && hasPermission(permissions.delete)
  const canDismiss = pending && hasPermission(permissions.update)
  const pair = t('queue.pairLabel', { first: candidate.record.title, second: candidate.duplicate.title })

  return (
    <li className="flex flex-col gap-3 rounded-lg border p-4 sm:flex-row sm:items-center sm:justify-between">
      <div className="grid min-w-0 gap-2">
        <p className="flex min-w-0 flex-wrap items-center gap-2 font-medium">
          <span className="truncate">{candidate.record.title}</span>
          <ArrowLeftRight aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
          <span className="truncate">{candidate.duplicate.title}</span>
        </p>
        <p className="text-sm break-words text-muted-foreground">
          {[candidate.record.subtitle, candidate.duplicate.subtitle].filter(Boolean).join(' · ')}
        </p>
        <div className="flex flex-wrap items-center gap-2">
          {candidate.rules.map((rule) => {
            const labelKey = ruleLabelKey(rule)
            return (
              <Badge key={rule} variant="secondary">
                {labelKey ? t(labelKey) : rule}
              </Badge>
            )
          })}
          <span className="text-xs text-muted-foreground">
            {t('queue.detectedAt', { date: formatDateTime(candidate.detected_at, formatting) })}
          </span>
        </div>
      </div>
      {(canMerge || canDismiss) && (
        <div className="flex flex-wrap gap-2">
          {canDismiss && (
            <Button
              size="sm"
              variant="ghost"
              disabled={dismiss.isPending}
              aria-label={`${t('queue.dismiss')}: ${pair}`}
              onClick={() =>
                dismiss.mutate(candidate.id, {
                  onSuccess: () => notifySuccess(t('queue.dismissed')),
                  onError: (error) => notifyError(error),
                })
              }
            >
              {t('queue.dismiss')}
            </Button>
          )}
          {canMerge && (
            <Button size="sm" asChild>
              <Link
                to={mergePagePath(recordType, candidate.record.id, candidate.duplicate.id)}
                aria-label={`${t('queue.compare')}: ${pair}`}
              >
                {t('queue.compare')}
              </Link>
            </Button>
          )}
        </div>
      )}
    </li>
  )
}
