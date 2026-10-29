import { ArrowLeft } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'
import { isApiError } from '@/api'
import { PageLoader } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { useApprovalRequest } from './api'
import { ApprovalSteps } from './components/ApprovalSteps'
import { RequestActions } from './components/RequestActions'
import { RequestComments, RequestSummary } from './components/RequestSummary'
import { ApprovalStatusBadge, OverdueBadge } from './components/StatusBadge'
import { APPROVALS_PATH } from './constants'
import { useRecordTypeLabel } from './lib/labels'
import { recordPath } from './lib/record-link'

export function ApprovalDetailPage() {
  const { t } = useTranslation('approvals')
  const { id } = useParams()
  const request = useApprovalRequest(id)
  const recordTypeLabel = useRecordTypeLabel()

  const back = (
    <Button asChild variant="ghost" size="sm" className="w-fit">
      <Link to={APPROVALS_PATH}>
        <ArrowLeft aria-hidden="true" />
        {t('detail.back')}
      </Link>
    </Button>
  )

  if (request.isPending) return <PageLoader />
  if (request.isError) {
    if (isApiError(request.error) && request.error.status === 404) {
      return (
        <div className="flex flex-col gap-4">
          {back}
          <EmptyState title={t('detail.notFoundTitle')} description={t('detail.notFoundDescription')} />
        </div>
      )
    }
    return <ErrorState onRetry={() => void request.refetch()} />
  }

  const data = request.data
  const link = recordPath(data.record_type, data.record_id)
  const title = data.record_title?.trim() || t('request.untitledRecord')

  return (
    <div className="flex w-full max-w-3xl flex-col gap-6">
      {back}
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="grid gap-1">
          <div className="flex flex-wrap items-center gap-2">
            <h1 className="text-xl font-semibold">{data.policy_name}</h1>
            <ApprovalStatusBadge status={data.status} />
            {data.is_overdue && data.status === 'pending' && <OverdueBadge />}
          </div>
          <p className="text-sm text-muted-foreground">
            {recordTypeLabel(data.record_type)} ·{' '}
            {link ? (
              <Link to={link} className="font-medium text-foreground hover:underline">
                {title}
              </Link>
            ) : (
              title
            )}
          </p>
        </div>
        <RequestActions request={data} size="default" />
      </div>
      <RequestSummary request={data} />
      <RequestComments request={data} />
      {data.locked_fields.length > 0 && data.status === 'pending' && (
        <p className="text-sm text-muted-foreground">
          {t('detail.lockedFields', { fields: data.locked_fields.join(', ') })}
        </p>
      )}
      <section className="grid gap-3">
        <h2 className="text-base font-semibold">{t('request.steps')}</h2>
        <ApprovalSteps steps={data.steps} />
      </section>
    </div>
  )
}
