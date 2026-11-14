import { PaginationSummary } from '@/components/ui/pagination-summary'
import { ArrowLeftRight, CopyCheck, Loader2, RefreshCw } from 'lucide-react'
import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { Link, useSearchParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { formatDateTime, getRecordResource, ListSkeleton, NativeSelect, SETTINGS_MANAGE, useRecordFormatting } from '@/sdk'
import { Pagination } from '@/components/ui/pagination'
import {
  CANDIDATE_STATUSES,
  DUPLICATES_PER_PAGE,
  duplicatesKeys,
  isCandidateStatus,
  isScanActive,
  useDismissDuplicate,
  useDuplicateCandidates,
  useDuplicateRules,
  useDuplicateScanStatus,
  useDuplicateTypes,
  useRequestDuplicateScan,
  type DuplicateCandidate,
  type DuplicateRecordType,
  type DuplicateScanStatus,
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
  const scanStatus = useDuplicateScanStatus()
  const scanning = isScanActive(scanStatus.data)
  const rules = useDuplicateRules(recordType, canManage)
  const queryClient = useQueryClient()
  const wasScanning = useRef(false)
  useEffect(() => {
    if (wasScanning.current && !scanning && scanStatus.data) {
      void queryClient.invalidateQueries({ queryKey: duplicatesKeys.all })
      if (scanStatus.data.status === 'completed') {
        notifySuccess(t('queue.scanDone', { count: scanStatus.data.candidates }))
      }
    }
    wasScanning.current = scanning
  }, [scanning, scanStatus.data, queryClient, t])

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

  const enabledRules = (rules.data?.rules ?? [])
    .filter((rule) => rule.enabled)
    .map((rule) => {
      const key = ruleLabelKey(rule.rule)
      return key ? t(key) : rule.rule
    })
  const emptyDescription =
    enabledRules.length > 0
      ? `${t('queue.emptyDescription')} ${t('queue.emptyRules', { rules: enabledRules.join(', ') })}`
      : t('queue.emptyDescription')

  return (
    <div className="grid gap-6">
      <Page>
        <PageHeader
          title={t('queue.title')}
          description={t('queue.description')}
          actions={
            canManage && (
              <Button size="sm" variant="outline" disabled={scan.isPending || scanning} onClick={requestScan}>
                {scanning ? <Loader2 className="animate-spin" aria-hidden="true" /> : <RefreshCw aria-hidden="true" />}
                {scanning ? t('queue.scanning') : t('queue.scan')}
              </Button>
            )
          }
        />
        <ScanLine status={scanStatus.data ?? null} loading={scanStatus.isPending} />
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
            description={status === 'pending' ? emptyDescription : undefined}
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
          <RecordTitle recordType={recordType} id={candidate.record.id} title={candidate.record.title} />
          <ArrowLeftRight aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
          <RecordTitle recordType={recordType} id={candidate.duplicate.id} title={candidate.duplicate.title} />
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
      {pending && !canMerge && !canDismiss && (
        <p className="max-w-xs text-xs text-muted-foreground">{t('queue.noPermission')}</p>
      )}
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

function RecordTitle({ recordType, id, title }: { recordType: string; id: string; title: string }) {
  const href = getRecordResource(recordType)?.detailPath?.(id)
  if (!href) return <span className="truncate">{title}</span>
  return (
    <Link to={href} className="truncate underline-offset-4 hover:underline" title={title}>
      {title}
    </Link>
  )
}

function ScanLine({ status, loading }: { status: DuplicateScanStatus | null; loading: boolean }) {
  const { t } = useTranslation('duplicates')
  const formatting = useRecordFormatting()
  if (loading) return null
  if (status === null) return <p className="text-sm text-muted-foreground">{t('queue.scanNever')}</p>
  if (isScanActive(status)) {
    return (
      <p className="flex items-center gap-2 text-sm text-muted-foreground" aria-live="polite">
        <Loader2 className="size-3.5 animate-spin" aria-hidden="true" />
        {t('queue.scanRunning')}
      </p>
    )
  }
  const when = formatDateTime(status.finished_at ?? status.requested_at, formatting)
  if (status.status === 'failed') {
    return <p className="text-sm text-destructive">{t('queue.scanFailed', { date: when })}</p>
  }
  return (
    <p className="text-sm text-muted-foreground">
      {t('queue.scanLast', { date: when, records: status.records, count: status.candidates })}
    </p>
  )
}
