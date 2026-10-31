import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'
import { isApiError } from '@/api'
import { PageLoader } from '@/components/feedback'
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
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'

export function ApprovalDetailPage() {
  const { t } = useTranslation('approvals')
  const { id } = useParams()
  const request = useApprovalRequest(id)
  const recordTypeLabel = useRecordTypeLabel()

  if (request.isPending) return <PageLoader />
  if (request.isError) {
    if (isApiError(request.error) && request.error.status === 404) {
      return (
        <Page>
          <PageHeader back={{ to: APPROVALS_PATH, label: t('detail.back') }} title={t('detail.notFoundTitle')} />
          <EmptyState title={t('detail.notFoundTitle')} description={t('detail.notFoundDescription')} />
        </Page>
      )
    }
    return <ErrorState onRetry={() => void request.refetch()} />
  }

  const data = request.data
  const link = recordPath(data.record_type, data.record_id)
  const title = data.record_title?.trim() || t('request.untitledRecord')

  return (
    <Page width="form">
      <PageHeader
        back={{ to: APPROVALS_PATH, label: t('detail.back') }}
        title={data.policy_name}
        badges={
          <>
            <ApprovalStatusBadge status={data.status} />
            {data.is_overdue && data.status === 'pending' && <OverdueBadge />}
          </>
        }
        description={
          <p>
            {recordTypeLabel(data.record_type)} ·{' '}
            {link ? (
              <Link to={link} className="font-medium text-foreground hover:underline">
                {title}
              </Link>
            ) : (
              title
            )}
          </p>
        }
        actions={<RequestActions request={data} size="default" />}
      />
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
    </Page>
  )
}
