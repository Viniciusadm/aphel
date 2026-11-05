import { useCallback, useState, type ReactNode } from 'react'
import { toast } from '@/components/ui/toast-store'
import i18n from '@/i18n'
import { approvalPendingDetails, approvalRequiredDetails } from '../errors'
import type { ApprovalRequiredDetails, ApprovalSubmission } from '../types'
import { RequestApprovalDialog } from './RequestApprovalDialog'

export interface ApprovalGate {
  handle: (error: unknown) => boolean
  dialog: ReactNode
}

export function notifyApprovalPending(error: unknown): boolean {
  const pending = approvalPendingDetails(error)
  if (!pending) return false
  toast({
    title: i18n.t('gate.pendingTitle', { ns: 'approvals' }),
    description: i18n.t('gate.pendingDescription', { ns: 'approvals' }),
  })
  return true
}

export function notifyApprovalGate(error: unknown): boolean {
  if (notifyApprovalPending(error)) return true
  if (!approvalRequiredDetails(error)) return false
  toast({
    title: i18n.t('errors.approval_required', { ns: 'approvals' }),
    variant: 'destructive',
  })
  return true
}

export function useApprovalGate(onSubmitted?: (result: ApprovalSubmission) => void): ApprovalGate {
  const [required, setRequired] = useState<ApprovalRequiredDetails | null>(null)

  const handle = useCallback((error: unknown) => {
    const details = approvalRequiredDetails(error)
    if (details) {
      setRequired(details)
      return true
    }
    return notifyApprovalPending(error)
  }, [])

  const dialog = required ? (
    <RequestApprovalDialog
      open
      onOpenChange={(open) => {
        if (!open) setRequired(null)
      }}
      recordType={required.record_type}
      recordId={required.record_id}
      moment={required.moment}
      targetId={required.target_id}
      onSubmitted={onSubmitted}
    />
  ) : null

  return { handle, dialog }
}
