import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import type { EnrollmentStatus } from '../types'

const STATUS_VARIANT: Record<EnrollmentStatus, 'success' | 'warning' | 'secondary' | 'outline'> = {
  active: 'success',
  paused: 'warning',
  completed: 'secondary',
  exited: 'outline',
}

export function SequenceStatusBadge({ active }: { active: boolean }) {
  const { t } = useTranslation('sequences')
  return <Badge variant={active ? 'success' : 'secondary'}>{t(active ? 'status.active' : 'status.inactive')}</Badge>
}

export function EnrollmentStatusBadge({ status }: { status: EnrollmentStatus }) {
  const { t } = useTranslation('sequences')
  return <Badge variant={STATUS_VARIANT[status]}>{t(`enrollmentStatus.${status}`)}</Badge>
}
