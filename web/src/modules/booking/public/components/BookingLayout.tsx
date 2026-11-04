import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'

interface BookingLayoutProps {
  children: ReactNode
}

export function BookingLayout({ children }: BookingLayoutProps) {
  const { t } = useTranslation('booking', { keyPrefix: 'public' })
  return (
    <div className="flex min-h-dvh flex-col bg-muted/40 text-foreground">
      <main className="flex flex-1 items-start justify-center px-4 py-8 sm:py-12">
        <div className="w-full max-w-4xl rounded-xl border bg-card text-card-foreground shadow-xs">{children}</div>
      </main>
      <footer className="pb-6 text-center text-xs text-muted-foreground">{t('poweredBy')}</footer>
    </div>
  )
}
