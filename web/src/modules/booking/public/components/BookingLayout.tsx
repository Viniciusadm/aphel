import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'

interface BookingLayoutProps {
  children: ReactNode
}

export function BookingLayout({ children }: BookingLayoutProps) {
  const { t } = useTranslation('booking', { keyPrefix: 'public' })
  return (
    <div className="flex min-h-dvh flex-col bg-muted/40 text-foreground">
      <main className="flex flex-1 items-start justify-center sm:items-center sm:px-6 sm:py-10">
        <div className="w-full bg-card text-card-foreground sm:max-w-5xl sm:overflow-hidden sm:rounded-2xl sm:border sm:shadow-sm">
          {children}
        </div>
      </main>
      <footer className="py-5 text-center text-xs text-muted-foreground">{t('poweredBy')}</footer>
    </div>
  )
}
