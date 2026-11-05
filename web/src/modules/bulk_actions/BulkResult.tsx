import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { CheckCircle2, Loader2, TriangleAlert } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Button } from '@/components/ui/button'
import { getRecordResource } from '@/sdk'
import { isFinished, useBulkItems, useBulkOperation, type BulkOperation } from './api'

export interface BulkResultProps {
  operation: BulkOperation
  recordType: string
}

export function BulkResult({ operation: initial, recordType }: BulkResultProps) {
  const { t } = useTranslation('bulk_actions')
  const query = useBulkOperation(initial)
  const operation = query.data ?? initial
  const items = useBulkItems(operation)
  const issues = items.data?.pages.flatMap((page) => page.data) ?? []
  const issuesTotal = items.data?.pages.at(-1)?.meta.total ?? 0
  const finished = isFinished(operation.status)
  const resource = getRecordResource(recordType)
  const percent = operation.total > 0 ? Math.round((operation.processed / operation.total) * 100) : 0

  return (
    <div className="grid gap-3" aria-live="polite">
      {!finished ? (
        <div className="grid gap-2">
          <p className="flex items-center gap-2 text-sm">
            <Loader2 className="size-4 animate-spin" aria-hidden="true" />
            {t(operation.status === 'queued' ? 'result.queued' : 'result.running', {
              processed: operation.processed,
              total: operation.total,
            })}
          </p>
          <div
            role="progressbar"
            aria-label={t('result.progress')}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={percent}
            className="h-2 overflow-hidden rounded-full bg-muted"
          >
            <div className="h-full bg-primary transition-all" style={{ width: `${percent}%` }} />
          </div>
          <p className="text-xs text-muted-foreground">{t('result.background')}</p>
        </div>
      ) : operation.status === 'failed' ? (
        <SharedFieldError className="flex items-center gap-2">
          <TriangleAlert className="size-4" aria-hidden="true" />
          {t('result.failed')}
        </SharedFieldError>
      ) : (
        <p className="flex items-center gap-2 text-sm">
          <CheckCircle2 className="size-4 text-success" aria-hidden="true" />
          {t('result.completed', { count: operation.succeeded })}
        </p>
      )}
      {finished && (
        <dl className="grid grid-cols-3 gap-2 text-center text-sm">
          <div className="rounded-md border p-2">
            <dt className="text-xs text-muted-foreground">{t('result.succeeded')}</dt>
            <dd className="font-semibold tabular-nums">{operation.succeeded}</dd>
          </div>
          <div className="rounded-md border p-2">
            <dt className="text-xs text-muted-foreground">{t('result.skipped')}</dt>
            <dd className="font-semibold tabular-nums">{operation.skipped}</dd>
          </div>
          <div className="rounded-md border p-2">
            <dt className="text-xs text-muted-foreground">{t('result.failedCount')}</dt>
            <dd className="font-semibold tabular-nums">{operation.failed}</dd>
          </div>
        </dl>
      )}
      {issues.length > 0 && (
        <div className="grid gap-1">
          <p className="text-sm font-medium">{t('result.reportTitle')}</p>
          <ul className="grid max-h-56 gap-1 overflow-y-auto text-sm" aria-label={t('result.reportTitle')}>
            {issues.map((item) => {
              const href = resource?.detailPath?.(item.record_id)
              return (
                <li key={item.record_id} className="grid gap-0.5 rounded-md border px-2 py-1.5">
                  {href ? (
                    <Link to={href} className="truncate font-mono text-xs underline-offset-4 hover:underline">
                      {item.record_id}
                    </Link>
                  ) : (
                    <span className="truncate font-mono text-xs">{item.record_id}</span>
                  )}
                  <span className="text-xs text-muted-foreground">{reasonLabel(t, item.code, item.message)}</span>
                </li>
              )
            })}
          </ul>
          {items.hasNextPage && (
            <div className="flex items-center justify-between gap-2">
              <p className="text-xs text-muted-foreground">
                {t('result.moreIssues', { count: issuesTotal - issues.length })}
              </p>
              <Button
                variant="outline"
                size="sm"
                disabled={items.isFetchingNextPage}
                onClick={() => void items.fetchNextPage()}
              >
                {t('result.loadMore')}
              </Button>
            </div>
          )}
        </div>
      )}
    </div>
  )
}

type Translate = ReturnType<typeof useTranslation<'bulk_actions'>>['t']

function reasonLabel(t: Translate, code: string | null | undefined, message: string | null | undefined): string {
  switch (code) {
    case 'permission_denied':
      return t('reasons.permissionDenied')
    case 'not_found':
    case 'contact_not_found':
    case 'company_not_found':
    case 'deal_not_found':
      return t('reasons.notFound')
    case 'invalid_fields':
      return t('reasons.invalid', { message: message ?? '' })
    case 'tag_not_allowed':
      return t('reasons.tagNotAllowed')
    case 'internal_error':
      return t('reasons.internal')
    case 'sequence_already_enrolled':
      return t('reasons.sequenceAlreadyEnrolled')
    case 'sequence_contact_without_email':
      return t('reasons.sequenceWithoutEmail')
    case 'sequence_contact_objected':
      return t('reasons.sequenceObjected')
    case 'no_mailbox_available':
      return t('reasons.noMailbox')
    default:
      return message ?? t('reasons.generic')
  }
}
