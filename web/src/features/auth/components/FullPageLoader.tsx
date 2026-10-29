import { LoaderCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'

export function FullPageLoader({ label }: { label?: string }) {
  const { t } = useTranslation('auth')
  return (
    <div role="status" className="flex min-h-screen items-center justify-center gap-2 bg-background text-sm text-muted-foreground">
      <LoaderCircle className="size-5 animate-spin" aria-hidden />
      <span>{label ?? t('session.loading')}</span>
    </div>
  )
}
