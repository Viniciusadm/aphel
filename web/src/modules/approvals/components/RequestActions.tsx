import { Ban, Check, X } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import type { ApprovalDecisionAction } from '../api'
import type { ApprovalRequest } from '../types'
import { DecisionDialog } from './DecisionDialog'

export interface RequestActionsProps {
  request: ApprovalRequest
  size?: 'sm' | 'default'
  onDone?: (request: ApprovalRequest) => void
}

export function RequestActions({ request, size = 'sm', onDone }: RequestActionsProps) {
  const { t } = useTranslation('approvals')
  const [action, setAction] = useState<ApprovalDecisionAction | null>(null)
  if (request.status !== 'pending' || (!request.can_decide && !request.can_cancel)) return null
  return (
    <div className="flex flex-wrap items-center gap-2">
      {request.can_decide && (
        <>
          <Button size={size} onClick={() => setAction('approve')}>
            <Check aria-hidden="true" />
            {t('actions.approve')}
          </Button>
          <Button size={size} variant="outline" onClick={() => setAction('reject')}>
            <X aria-hidden="true" />
            {t('actions.reject')}
          </Button>
        </>
      )}
      {request.can_cancel && (
        <Button size={size} variant="ghost" onClick={() => setAction('cancel')}>
          <Ban aria-hidden="true" />
          {t('actions.cancel')}
        </Button>
      )}
      <DecisionDialog
        request={request}
        action={action}
        onOpenChange={(open) => {
          if (!open) setAction(null)
        }}
        onDone={onDone}
      />
    </div>
  )
}
