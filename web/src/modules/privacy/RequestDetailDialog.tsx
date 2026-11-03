import { Download } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDate, formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import {
  requestDownloadUrl,
  usePrivacyRequest,
  useRequestMutations,
  type PrivacyRequest,
  type PrivacyRequestDetail,
} from './api'
import { localizedLabel, usePrivacyErrors } from './lib/shared'
import { RecordLink } from './RecordLink'
import { StatusBadge } from './StatusBadge'
import { Alert } from '@/components/ui/alert'

const REPORT_KEYS = [
  'records',
  'attachments',
  'files',
  'history_entries',
  'external_items',
  'skipped',
  'errors',
] as const

interface RequestDetailDialogProps {
  id: string
  onClose: () => void
}

export function RequestDetailDialog({ id, onClose }: RequestDetailDialogProps) {
  const { t } = useTranslation('privacy')
  const detail = usePrivacyRequest(id)

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent size="xl">
        <DialogHeader>
          <DialogTitle>{t('detail.title')}</DialogTitle>
          <DialogDescription>{t('detail.description')}</DialogDescription>
        </DialogHeader>
        {detail.isPending ? (
          <div className="grid gap-2" aria-busy="true">
            <Skeleton className="h-6 w-1/2" />
            <Skeleton className="h-24 w-full" />
          </div>
        ) : detail.isError ? (
          <ErrorState onRetry={() => void detail.refetch()} />
        ) : (
          <RequestDetailBody detail={detail.data} onClose={onClose} />
        )}
      </DialogContent>
    </Dialog>
  )
}

function RequestDetailBody({ detail, onClose }: { detail: PrivacyRequestDetail; onClose: () => void }) {
  const { t } = useTranslation('privacy')
  const formatting = useRecordFormatting()
  const { request } = detail
  const { execute, cancel } = useRequestMutations()
  const notifyFailure = usePrivacyErrors()
  const [confirming, setConfirming] = useState<'execute' | 'cancel' | null>(null)
  const pending = request.status === 'pending'
  const destructive = request.kind !== 'export'

  const runExecute = async () => {
    try {
      await execute.mutateAsync(request.id)
      notifySuccess(t('detail.executed'))
    } catch (error) {
      notifyFailure(error)
    }
  }

  const runCancel = async () => {
    try {
      await cancel.mutateAsync(request.id)
      notifySuccess(t('detail.cancelled'))
    } catch (error) {
      notifyFailure(error)
    }
  }

  return (
    <div className="grid min-w-0 gap-4">
      <dl className="grid gap-3 text-sm sm:grid-cols-2">
        <Info label={t('detail.kind')}>{t(`kinds.${request.kind as 'export'}`)}</Info>
        <Info label={t('detail.status')}>
          <span className="flex flex-wrap items-center gap-2">
            <StatusBadge status={request.status} />
            {request.overdue && <Badge variant="destructive">{t('requests.overdue')}</Badge>}
          </span>
        </Info>
        <Info label={t('detail.subject')}>
          <span className="break-all">
            {request.subject_label} ({t(`subjectKinds.${request.subject_kind as 'email'}`)})
          </span>
        </Info>
        <Info label={t('detail.dueAt')}>{formatDate(request.due_at, formatting)}</Info>
        <Info label={t('detail.requestedBy')}>
          {request.requested_by?.display_name || t('detail.unknownUser')} ·{' '}
          {formatDateTime(request.created_at, formatting)}
        </Info>
        {request.executed_by && (
          <Info label={t('detail.executedBy')}>
            {request.executed_by.display_name || t('detail.unknownUser')}
            {request.started_at ? ` · ${formatDateTime(request.started_at, formatting)}` : ''}
          </Info>
        )}
        {request.finished_at && (
          <Info label={t('detail.finishedAt')}>{formatDateTime(request.finished_at, formatting)}</Info>
        )}
      </dl>

      {request.failure_reason && (
        <Alert>
          {t('detail.failure')}
        </Alert>
      )}

      {request.result && <Report request={request} sources={detail.sources} />}

      <section className="grid gap-2">
        <h3 className="text-sm font-medium">{t('detail.targets')}</h3>
        {detail.records.length === 0 && detail.items.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t('detail.noTargets')}</p>
        ) : (
          <ul className="grid gap-1 text-sm">
            {detail.records.map((record) => (
              <li key={`${record.record_type}:${record.id}`} className="flex min-w-0 flex-wrap items-center gap-2">
                <Badge variant="outline">{record.record_type}</Badge>
                {record.exists ? (
                  <RecordLink recordType={record.record_type} id={record.id} title={record.title || record.id} />
                ) : (
                  <span className="text-muted-foreground">{t('detail.removedRecord')}</span>
                )}
              </li>
            ))}
            {detail.items.map((item) => <li key={JSON.stringify([item.source, item.id])} className="text-muted-foreground">
              {item.source}: {item.id}
            </li>)}
          </ul>
        )}
      </section>

      <DialogFooter className="flex-wrap gap-2">
        <Button variant="outline" onClick={onClose}>
          {t('actions.close')}
        </Button>
        {request.download_available && (
          <Button asChild variant="outline">
            <a href={requestDownloadUrl(request.id)} download>
              <Download aria-hidden="true" />
              {t('actions.download')}
            </a>
          </Button>
        )}
        {pending && (
          <>
            <Button variant="outline" onClick={() => setConfirming('cancel')}>
              {t('actions.cancelRequest')}
            </Button>
            <Button variant={destructive ? 'destructive' : 'default'} onClick={() => setConfirming('execute')}>
              {t(`execute.${request.kind as 'export'}`)}
            </Button>
          </>
        )}
      </DialogFooter>

      <ConfirmDialog
        open={confirming === 'execute'}
        onOpenChange={(open) => !open && setConfirming(null)}
        title={t(`execute.${request.kind as 'export'}`)}
        description={t(`executeConfirm.${request.kind as 'export'}`)}
        confirmLabel={t(`execute.${request.kind as 'export'}`)}
        variant={destructive ? 'destructive' : 'default'}
        confirmationText={destructive ? t(`confirmWords.${request.kind as 'anonymize'}`) : undefined}
        notifyOnError={false}
        onConfirm={runExecute}
      />
      <ConfirmDialog
        open={confirming === 'cancel'}
        onOpenChange={(open) => !open && setConfirming(null)}
        title={t('actions.cancelRequest')}
        description={t('detail.cancelConfirm')}
        confirmLabel={t('actions.cancelRequest')}
        notifyOnError={false}
        onConfirm={runCancel}
      />
    </div>
  )
}

function Info({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid min-w-0 gap-0.5">
      <dt className="text-xs text-muted-foreground">{label}</dt>
      <dd className="min-w-0">{children}</dd>
    </div>
  )
}

function Report({ request, sources }: { request: PrivacyRequest; sources: PrivacyRequestDetail['sources'] }) {
  const { t, i18n } = useTranslation('privacy')
  const result = request.result
  if (!result) return null
  return (
    <section className="grid gap-2">
      <h3 className="text-sm font-medium">{t('detail.report')}</h3>
      <dl className="grid grid-cols-2 gap-2 sm:grid-cols-3">
        {REPORT_KEYS.filter((key) => result[key] > 0 || key === 'records').map((key) => (
          <div key={key} className="rounded-md border px-3 py-2">
            <dt className="text-xs text-muted-foreground">{t(`report.${key}`)}</dt>
            <dd className="text-lg font-semibold tabular-nums">{result[key]}</dd>
          </div>
        ))}
        {Object.entries(result.source_items).map(([key, count]) => <div key={key} className="rounded-md border px-3 py-2">
          <dt className="text-xs text-muted-foreground">{localizedLabel(sources.find((source) => source.key === key)?.labels ?? {}, i18n.language, key)}</dt>
          <dd className="text-lg font-semibold tabular-nums">{count}</dd>
        </div>)}
      </dl>
    </section>
  )
}
