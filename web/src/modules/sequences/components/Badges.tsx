import { useTranslation } from 'react-i18next'
import { Badge, type BadgeTone } from '@/components/ui/badge'
import type { EnrollmentStatus } from '../types'

const STATUS_TONE: Record<EnrollmentStatus, BadgeTone> = {
  active: 'success',
  paused: 'warning',
  completed: 'primary',
  exited: 'neutral',
}

export function SequenceStatusBadge({ active }: { active: boolean }) {
  const { t } = useTranslation('sequences')
  return <Badge tone={active ? 'success' : 'neutral'}>{t(active ? 'status.active' : 'status.inactive')}</Badge>
}

export function EnrollmentStatusBadge({ status }: { status: EnrollmentStatus }) {
  const { t } = useTranslation('sequences')
  return <Badge tone={STATUS_TONE[status]}>{t(`enrollmentStatus.${status}`)}</Badge>
}
