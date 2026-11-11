import { useTranslation } from 'react-i18next'
import type { BadgeTone } from '@/components/ui/badge'
import { StatusBadge } from '@/components/ui/status-badge'
import type { TenantStatus } from '../api'

const TONES: Record<TenantStatus, BadgeTone> = {
  provisioning: 'primary',
  active: 'success',
  suspended: 'warning',
  deleting: 'destructive',
}

export function TenantStatusBadge({ status }: { status: string }) {
  const { t } = useTranslation('console')
  const known = status as TenantStatus
  return <StatusBadge tone={TONES[known] ?? 'neutral'}>{t(`status.${known}`)}</StatusBadge>
}

