import { LoaderCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'

export interface PageLoaderProps {
  label?: string
  fullScreen?: boolean
  className?: string
}

export function PageLoader({ label, fullScreen = false, className }: PageLoaderProps) {
  const { t } = useTranslation('records-detail')
  const text = label ?? t('pageLoader')
  return (
    <div
      role="status"
      aria-live="polite"
      className={cn(
        'flex flex-col items-center justify-center gap-3 py-16 text-muted-foreground',
        fullScreen && 'min-h-dvh py-0',
        className,
      )}
    >
      <LoaderCircle className="size-6 animate-spin" aria-hidden="true" />
      <span className="text-sm">{text}</span>
    </div>
  )
}
