import { useTranslation } from 'react-i18next'
import { Badge, type BadgeTone } from '@/components/ui/badge'
import type { ApprovalStatus, ApprovalStepState } from '../types'

const STATUS_VARIANTS: Record<ApprovalStatus, BadgeTone> = {
  pending: 'warning',
  approved: 'success',
  rejected: 'destructive',
  cancelled: 'neutral',
}

const STEP_VARIANTS: Record<ApprovalStepState, BadgeTone> = {
  waiting: 'neutral',
  active: 'warning',
  approved: 'success',
  rejected: 'destructive',
  skipped: 'neutral',
}

export function ApprovalStatusBadge({ status }: { status: ApprovalStatus }) {
  const { t } = useTranslation('approvals')
  return <Badge tone={STATUS_VARIANTS[status]}>{t(`status.${status}`)}</Badge>
}

export function ApprovalStepBadge({ state }: { state: ApprovalStepState }) {
  const { t } = useTranslation('approvals')
  return <Badge tone={STEP_VARIANTS[state]}>{t(`stepState.${state}`)}</Badge>
}

export function OverdueBadge() {
  const { t } = useTranslation('approvals')
  return <Badge tone="destructive">{t('request.overdue')}</Badge>
}
