import { PageTitle } from '@/components/ui/page-title'
import { ArrowLeft } from 'lucide-react'
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

export interface RecordBackLink {
  to: string
  label?: string
}

export interface RecordHeaderProps {
  title: ReactNode
  titleAccessory?: ReactNode
  subtitle?: ReactNode
  owner?: ReactNode
  actions?: ReactNode
  back?: RecordBackLink
  className?: string
}

export function RecordHeader({ title, titleAccessory, subtitle, owner, actions, back, className }: RecordHeaderProps) {
  const { t } = useTranslation('records-detail')
  return (
    <header className={cn('flex flex-wrap items-start justify-between gap-x-4 gap-y-3', className)}>
      <div className="flex min-w-[min(100%,14rem)] flex-1 items-start gap-2">
        {back && (
          <Button variant="ghost" size="icon" asChild className="-ml-2 shrink-0">
            <Link to={back.to} aria-label={back.label ?? t('header.back')}>
              <ArrowLeft aria-hidden="true" />
            </Link>
          </Button>
        )}
        <div className="grid min-w-0 gap-1">
          <div className="flex min-w-0 items-start gap-1">
            <PageTitle className="min-w-0 break-words">{title}</PageTitle>
            {titleAccessory}
          </div>
          {(subtitle || owner) && (
            <div className="flex min-w-0 flex-wrap items-center gap-x-4 gap-y-1 text-sm text-muted-foreground">
              {subtitle && <div className="min-w-0 max-w-full truncate">{subtitle}</div>}
              {owner && (
                <div className="flex min-w-0 items-center gap-2">
                  <span>{t('header.owner')}:</span>
                  <span className="min-w-0 text-foreground">{owner}</span>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
      {actions && <div className="flex w-full flex-wrap items-center gap-2 sm:w-auto">{actions}</div>}
    </header>
  )
}
