import { Orbit } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'

export function Brand({ compact = false }: { compact?: boolean }) {
  const { t } = useTranslation()
  return (
    <div className={cn('flex items-center gap-2 font-semibold', compact && 'justify-center')}>
      <span className="flex size-8 items-center justify-center rounded-md bg-primary-solid text-primary-foreground">
        <Orbit className="size-5" aria-hidden />
      </span>
      {!compact && <span className="text-base tracking-tight">{t('appName')}</span>}
    </div>
  )
}
