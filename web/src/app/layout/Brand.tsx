import { Orbit } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'

interface BrandProps {
  compact?: boolean
  tone?: 'default' | 'inverted'
  label?: string
}

export function Brand({ compact = false, tone = 'default', label }: BrandProps) {
  const { t } = useTranslation()
  return (
    <div className={cn('flex items-center gap-2 font-semibold', compact && 'justify-center')}>
      <span
        className={cn(
          'flex size-8 items-center justify-center rounded-md',
          tone === 'inverted' ? 'bg-primary-foreground text-primary-solid shadow-sm' : 'bg-primary-solid text-primary-foreground',
        )}
      >
        <Orbit className="size-5" aria-hidden />
      </span>
      {!compact && <span className="text-base tracking-tight">{label ?? t('appName')}</span>}
    </div>
  )
}
