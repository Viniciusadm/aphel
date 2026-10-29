import { useTranslation } from 'react-i18next'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import type { AutomationStatus, RunStatus, StepStatus } from '../types'

type Variant = BadgeProps['variant']

const AUTOMATION_VARIANTS: Record<AutomationStatus, Variant> = {
  draft: 'secondary',
  active: 'success',
  paused: 'warning',
}

const RUN_VARIANTS: Record<RunStatus, Variant> = {
  running: 'default',
  waiting: 'warning',
  succeeded: 'success',
  failed: 'destructive',
  skipped: 'secondary',
  cancelled: 'outline',
}

const STEP_VARIANTS: Record<StepStatus, Variant> = {
  succeeded: 'success',
  failed: 'destructive',
  skipped: 'secondary',
  planned: 'outline',
  waiting: 'warning',
}

export function AutomationStatusBadge({ status }: { status: AutomationStatus }) {
  const { t } = useTranslation('automations')
  return <Badge variant={AUTOMATION_VARIANTS[status] ?? 'outline'}>{t(`status.${status}`, { defaultValue: status })}</Badge>
}

export function RunStatusBadge({ status }: { status: RunStatus }) {
  const { t } = useTranslation('automations')
  return <Badge variant={RUN_VARIANTS[status] ?? 'outline'}>{t(`runStatus.${status}`, { defaultValue: status })}</Badge>
}

export function StepStatusBadge({ status }: { status: StepStatus }) {
  const { t } = useTranslation('automations')
  return <Badge variant={STEP_VARIANTS[status] ?? 'outline'}>{t(`stepStatus.${status}`, { defaultValue: status })}</Badge>
}
