import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { usePendingApprovalsCount } from '../api'

export function ApprovalsNavBadge({ collapsed = false }: { collapsed?: boolean }) {
  const { t } = useTranslation('approvals')
  const count = usePendingApprovalsCount()
  const value = count.data ?? 0
  if (value <= 0) return null
  const label = value > 99 ? '99+' : String(value)
  return (
    <span
      aria-label={t('nav.pending', { count: value })}
      className={cn(
        'inline-flex min-w-5 items-center justify-center rounded-full bg-primary-solid px-1.5 text-[0.65rem] leading-5 font-semibold text-primary-foreground',
        collapsed ? 'absolute top-0.5 right-0.5 min-w-4 px-1 leading-4' : 'ml-auto',
      )}
    >
      {label}
    </span>
  )
}
