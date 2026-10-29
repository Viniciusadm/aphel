import { useTranslation } from 'react-i18next'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import type { MemberStatus } from '../api'

const variants: Record<MemberStatus, BadgeProps['variant']> = {
  invited: 'warning',
  active: 'success',
  deactivated: 'destructive',
}

export function MemberStatusBadge({ status }: { status: MemberStatus }) {
  const { t } = useTranslation('settings')
  return <Badge variant={variants[status]}>{t(`users.status.${status}`)}</Badge>
}
