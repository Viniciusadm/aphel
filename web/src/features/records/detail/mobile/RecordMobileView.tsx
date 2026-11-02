import { ChevronLeft, ChevronRight, MoreHorizontal, Pencil } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'
import { Avatar } from '@/components/ui/avatar'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { Page } from '@/components/ui/page'
import { cn } from '@/lib/utils'
import type { RecordBackLink } from '../RecordHeader'
import { RECORD_TAB_PARAM } from '../use-active-tab'
import type { RecordTab } from '../types'

export interface RecordMobileViewProps {
  title: string
  titleAccessory?: ReactNode
  headerSlot?: ReactNode
  subtitle?: ReactNode
  owner?: ReactNode
  back?: RecordBackLink
  tabs: RecordTab[]
  overview?: ReactNode
  summaryPanel: ReactNode
  detailsPanel: ReactNode
  actions?: ReactNode
  onEdit?: () => void
  className?: string
}

const SHEET_ACTIONS =
  'grid gap-2 px-4 pt-4 pb-[max(1rem,env(safe-area-inset-bottom))] [&_button]:min-h-11 [&_button]:w-full [&_button]:justify-start [&_div]:w-full [&_a]:min-h-11 [&_a]:w-full'

function BottomSheet({
  open,
  onOpenChange,
  title,
  children,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: string
  children: ReactNode
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent side="bottom">
        <div className="sticky top-0 z-10 border-b bg-background px-4 py-3 pr-12">
          <DialogTitle className="truncate text-base">{title}</DialogTitle>
          <DialogDescription className="sr-only">{title}</DialogDescription>
        </div>
        {children}
      </DialogContent>
    </Dialog>
  )
}

export function RecordMobileView({
  title,
  titleAccessory,
  headerSlot,
  subtitle,
  owner,
  back,
  tabs,
  overview,
  summaryPanel,
  detailsPanel,
  actions,
  onEdit,
  className,
}: RecordMobileViewProps) {
  const { t } = useTranslation('records-detail')
  const [searchParams, setSearchParams] = useSearchParams()
  const [actionsOpen, setActionsOpen] = useState(false)
  const [detailsOpen, setDetailsOpen] = useState(false)
  const requested = searchParams.get(RECORD_TAB_PARAM)
  const activeTab = tabs.find((tab) => tab.key === requested) ?? null

  const closeTab = () =>
    setSearchParams(
      (current) => {
        const next = new URLSearchParams(current)
        next.delete(RECORD_TAB_PARAM)
        return next
      },
      { replace: true },
    )

  if (activeTab) {
    return (
      <Page className={cn('gap-4', className)}>
        <div className="flex items-center gap-1 border-b pb-3">
          <Button variant="ghost" size="icon" className="-ml-2 size-11" onClick={closeTab} aria-label={t('mobile.back')}>
            <ChevronLeft aria-hidden="true" />
          </Button>
          <div className="grid min-w-0">
            <span className="truncate text-xs text-muted-foreground">{title}</span>
            <h1 className="truncate text-lg font-semibold">{activeTab.label}</h1>
          </div>
        </div>
        <div className="min-w-0">{activeTab.content}</div>
      </Page>
    )
  }

  return (
    <Page className={cn('gap-4', className)}>
      {back && (
        <Link
          to={back.to}
          className="-mt-1 inline-flex min-h-11 w-fit items-center gap-1 text-sm text-muted-foreground hover:text-foreground"
        >
          <ChevronLeft className="size-4" aria-hidden="true" />
          {back.label ?? t('header.back')}
        </Link>
      )}
      <header className="flex items-start gap-4">
        <Avatar name={title} size="sm" />
        <div className="grid min-w-0 flex-1 gap-1">
          <h1 className="text-xl leading-tight font-semibold break-words">{title}</h1>
          {titleAccessory && <div className="flex flex-wrap items-center gap-2">{titleAccessory}</div>}
          {subtitle && <div className="min-w-0 truncate text-sm text-muted-foreground">{subtitle}</div>}
          {owner && (
            <div className="flex min-w-0 items-center gap-1.5 text-sm text-muted-foreground">
              <span>{t('header.owner')}:</span>
              <span className="min-w-0 text-foreground">{owner}</span>
            </div>
          )}
        </div>
      <div className="flex shrink-0 gap-1">
        {onEdit && (
          <Button variant="outline" size="icon" aria-label={t('header.edit')} onClick={onEdit}>
            <Pencil aria-hidden="true" />
          </Button>
        )}
        <Button variant="outline" size="icon" aria-label={t('mobile.actions')} onClick={() => setActionsOpen(true)}>
          <MoreHorizontal aria-hidden="true" />
        </Button>
      </div>
      </header>
      {headerSlot}
      {overview ?? summaryPanel}
      {!overview && <button
        type="button"
        onClick={() => setDetailsOpen(true)}
        className="flex min-h-12 items-center justify-between rounded-lg border bg-card px-4 text-sm font-medium focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
      >
        {t('mobile.allDetails')}
        <ChevronRight className="size-4 text-muted-foreground" aria-hidden="true" />
      </button>}
      <BottomSheet open={actionsOpen} onOpenChange={setActionsOpen} title={t('mobile.actions')}>
        <div className={SHEET_ACTIONS}>{actions}</div>
      </BottomSheet>
      <BottomSheet open={detailsOpen} onOpenChange={setDetailsOpen} title={t('fields.title')}>
        <div className="px-4 pt-4 pb-[max(1rem,env(safe-area-inset-bottom))]">{detailsPanel}</div>
      </BottomSheet>
    </Page>
  )
}
