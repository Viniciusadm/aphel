import { MoreHorizontal } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { useState, type ReactNode } from 'react'
import { RecordActionsContext } from './actions-context'
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
  secondaryAction?: ReactNode
  actions?: ReactNode
  back?: RecordBackLink
  className?: string
}

export function RecordHeader({ title, titleAccessory, subtitle, owner, actions, primaryAction, secondaryAction, back, className }: RecordHeaderProps) {
  const { t } = useTranslation('records-detail')
  const [open, setOpen] = useState(false)
  return (
    <PageHeader
      className={className}
      title={title}
      badges={titleAccessory}
      back={back && { ...back, label: back.label ?? t('header.back') }}
      actions={<>{primaryAction}{secondaryAction}{actions && <Popover open={open} onOpenChange={setOpen}><PopoverTrigger asChild><Button variant="outline" size="sm"><MoreHorizontal aria-hidden />{t('mobile.actions')}</Button></PopoverTrigger><PopoverContent align="end" className="grid min-w-56 gap-1 p-1 [&_a]:min-h-11 [&_[data-slot=button]]:min-h-11 [&_[data-slot=button]]:w-full [&_[data-slot=button]]:justify-start [&_[data-slot=button]]:rounded-md [&_[data-slot=button]]:border-0 [&_[data-slot=button]]:bg-transparent [&_[data-slot=button]]:px-3 [&_[data-slot=button]]:font-normal [&_[data-slot=button]]:shadow-none [&_[data-slot=button]:hover]:bg-accent [&_div:has(>[data-slot=button])]:contents [&_[data-destructive]]:order-last [&_[data-destructive]]:text-destructive [&_[data-destructive]:hover]:!bg-destructive/10"><RecordActionsContext.Provider value={{ close: () => setOpen(false) }}>{actions}</RecordActionsContext.Provider></PopoverContent></Popover>}</>}
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
