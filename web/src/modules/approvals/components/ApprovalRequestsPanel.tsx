import { Plus } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useRecordApprovalRequests } from '../api'
import { approvalPath } from '../constants'
import { ApprovalSteps } from './ApprovalSteps'
import { RequestActions } from './RequestActions'
import { RequestApprovalDialog } from './RequestApprovalDialog'
import { RequestComments, RequestSummary } from './RequestSummary'
import { ApprovalStatusBadge, OverdueBadge } from './StatusBadge'

export interface ApprovalRequestsPanelProps {
  recordType: string
  recordId: string
}

export function ApprovalRequestsPanel({ recordType, recordId }: ApprovalRequestsPanelProps) {
  const { t } = useTranslation('approvals')
  const requests = useRecordApprovalRequests(recordType, recordId)
  const [requesting, setRequesting] = useState(false)

  let body
  if (requests.isPending) {
    body = (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-20 w-full" />
        <Skeleton className="h-20 w-full" />
      </div>
    )
  } else if (requests.isError) {
    body = <ErrorState onRetry={() => void requests.refetch()} />
  } else if (requests.data.length === 0) {
    body = <p className="text-sm text-muted-foreground">{t('panel.empty')}</p>
  } else {
    body = (
      <ul className="grid gap-3">
        {requests.data.map((request) => (
          <li key={request.id} className="grid gap-3 rounded-lg border bg-card p-3">
            <div className="flex flex-wrap items-center gap-2">
              <Link to={approvalPath(request.id)} className="font-medium hover:underline">
                {request.policy_name}
              </Link>
              <ApprovalStatusBadge status={request.status} />
              {request.is_overdue && request.status === 'pending' && <OverdueBadge />}
            </div>
            <RequestSummary request={request} />
            <RequestComments request={request} />
            <ApprovalSteps steps={request.steps} compact />
            <RequestActions request={request} />
          </li>
        ))}
      </ul>
    )
  }

  return (
    <section className="grid gap-3" aria-label={t('panel.title')}>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h3 className="text-sm font-semibold">{t('panel.title')}</h3>
        <Button size="sm" variant="outline" onClick={() => setRequesting(true)}>
          <Plus aria-hidden="true" />
          {t('panel.request')}
        </Button>
      </div>
      {body}
      <RequestApprovalDialog
        open={requesting}
        onOpenChange={setRequesting}
        recordType={recordType}
        recordId={recordId}
        moment="on_demand"
      />
    </section>
  )
}
