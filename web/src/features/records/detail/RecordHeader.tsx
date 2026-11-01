import { MoreHorizontal } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { PageHeader } from '@/components/ui/page-header'

export interface RecordBackLink {
  to: string
  label?: string
}

export interface RecordHeaderProps {
  title: ReactNode
  titleAccessory?: ReactNode
  subtitle?: ReactNode
  owner?: ReactNode
  primaryAction?: ReactNode
  actions?: ReactNode
  back?: RecordBackLink
  className?: string
}

export function RecordHeader({ title, titleAccessory, subtitle, owner, actions, primaryAction, back, className }: RecordHeaderProps) {
  const { t } = useTranslation('records-detail')
  return (
    <PageHeader
      className={className}
      title={title}
      badges={titleAccessory}
      back={back && { ...back, label: back.label ?? t('header.back') }}
      actions={<>{primaryAction}{actions && <Popover><PopoverTrigger asChild><Button variant="outline" size="sm"><MoreHorizontal aria-hidden />{t('mobile.actions')}</Button></PopoverTrigger><PopoverContent align="end" className="grid gap-2 [&_button]:min-h-11 [&_button]:justify-start [&_button]:whitespace-normal [&_a]:min-h-11">{actions}</PopoverContent></Popover>}</>}
      description={(subtitle || owner) && (
        <div className="flex min-w-0 flex-wrap items-center gap-x-4 gap-y-1">
          {subtitle && <div className="min-w-0 max-w-full truncate">{subtitle}</div>}
          {owner && <div className="flex min-w-0 items-center gap-2">
            <span>{t('header.owner')}:</span>
            <span className="min-w-0 text-foreground">{owner}</span>
          </div>}
        </div>
      )}
    />
  )
}
