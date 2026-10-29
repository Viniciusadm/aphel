import { useTranslation } from 'react-i18next'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import type { ApprovalRequest } from '../types'

export function RequestSummary({ request }: { request: ApprovalRequest }) {
  const { t } = useTranslation('approvals')
  const formatting = useRecordFormatting()
  return (
    <dl className="grid gap-x-4 gap-y-1 text-sm sm:grid-cols-[auto_minmax(0,1fr)]">
      <dt className="text-muted-foreground">{t('request.moment')}</dt>
      <dd>
        {t(`moments.${request.moment}`)}
        {request.target_name && ` · ${request.target_name}`}
      </dd>
      <dt className="text-muted-foreground">{t('request.requestedBy')}</dt>
      <dd>
        {request.requested_by?.display_name ?? t('request.system')} · {formatDateTime(request.requested_at, formatting)}
      </dd>
      {request.due_at && (
        <>
          <dt className="text-muted-foreground">{t('request.dueAt')}</dt>
          <dd
            className={request.is_overdue && request.status === 'pending' ? 'font-medium text-destructive' : undefined}
          >
            {formatDateTime(request.due_at, formatting)}
          </dd>
        </>
      )}
      {request.resolved_at && (
        <>
          <dt className="text-muted-foreground">{t('request.resolvedAt')}</dt>
          <dd>
            {request.resolved_by ? `${request.resolved_by.display_name} · ` : ''}
            {formatDateTime(request.resolved_at, formatting)}
          </dd>
        </>
      )}
    </dl>
  )
}

export function RequestComments({ request }: { request: ApprovalRequest }) {
  const { t } = useTranslation('approvals')
  if (!request.comment && !request.resolution_comment) return null
  return (
    <div className="grid gap-2 text-sm">
      {request.comment && (
        <div className="grid gap-0.5">
          <span className="text-xs font-medium text-muted-foreground">{t('request.comment')}</span>
          <p className="whitespace-pre-line">{request.comment}</p>
        </div>
      )}
      {request.resolution_comment && (
        <div className="grid gap-0.5">
          <span className="text-xs font-medium text-muted-foreground">{t('request.resolutionComment')}</span>
          <p className="whitespace-pre-line">{request.resolution_comment}</p>
        </div>
      )}
    </div>
  )
}
