import { useTranslation } from 'react-i18next'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import type { ApprovalStatus, ApprovalStepState } from '../types'

const STATUS_VARIANTS: Record<ApprovalStatus, BadgeProps['variant']> = {
  pending: 'warning',
  approved: 'success',
  rejected: 'destructive',
  cancelled: 'secondary',
}

const STEP_VARIANTS: Record<ApprovalStepState, BadgeProps['variant']> = {
  waiting: 'outline',
  active: 'warning',
  approved: 'success',
  rejected: 'destructive',
  skipped: 'secondary',
}

export function ApprovalStatusBadge({ status }: { status: ApprovalStatus }) {
  const { t } = useTranslation('approvals')
  return <Badge variant={STATUS_VARIANTS[status]}>{t(`status.${status}`)}</Badge>
}

export function ApprovalStepBadge({ state }: { state: ApprovalStepState }) {
  const { t } = useTranslation('approvals')
  return <Badge variant={STEP_VARIANTS[state]}>{t(`stepState.${state}`)}</Badge>
}

export function OverdueBadge() {
  const { t } = useTranslation('approvals')
  return <Badge variant="destructive">{t('request.overdue')}</Badge>
}
