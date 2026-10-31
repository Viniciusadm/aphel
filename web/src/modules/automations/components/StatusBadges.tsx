import { useTranslation } from 'react-i18next'
import { Badge, type BadgeTone } from '@/components/ui/badge'
import type { AutomationStatus, RunStatus, StepStatus } from '../types'

const AUTOMATION_VARIANTS: Record<AutomationStatus, BadgeTone> = {
  draft: 'neutral',
  active: 'success',
  paused: 'warning',
}

const RUN_VARIANTS: Record<RunStatus, BadgeTone> = {
  running: 'primary',
  waiting: 'warning',
  succeeded: 'success',
  failed: 'destructive',
  skipped: 'neutral',
  cancelled: 'neutral',
}

const STEP_VARIANTS: Record<StepStatus, BadgeTone> = {
  succeeded: 'success',
  failed: 'destructive',
  skipped: 'neutral',
  planned: 'neutral',
  waiting: 'warning',
}

export function AutomationStatusBadge({ status }: { status: AutomationStatus }) {
  const { t } = useTranslation('automations')
  return <Badge tone={AUTOMATION_VARIANTS[status] ?? 'neutral'}>{t(`status.${status}`, { defaultValue: status })}</Badge>
}

export function RunStatusBadge({ status }: { status: RunStatus }) {
  const { t } = useTranslation('automations')
  return <Badge tone={RUN_VARIANTS[status] ?? 'neutral'}>{t(`runStatus.${status}`, { defaultValue: status })}</Badge>
}

export function StepStatusBadge({ status }: { status: StepStatus }) {
  const { t } = useTranslation('automations')
  return <Badge tone={STEP_VARIANTS[status] ?? 'neutral'}>{t(`stepStatus.${status}`, { defaultValue: status })}</Badge>
}
