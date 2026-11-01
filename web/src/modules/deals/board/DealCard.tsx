import { useDraggable } from '@dnd-kit/core'
import { Building2, Clock, EllipsisVertical, Shuffle, User } from 'lucide-react'
import { memo, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useLocation } from 'react-router'
import { Avatar } from '@/components/ui/avatar'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { formatCurrency, type RecordFormatting } from '@/features/records/formatting'
import { useFilters } from '@/kernel/modules/filters'
import { Slot } from '@/kernel/modules/Slot'
import { cn } from '@/lib/utils'
import { dealPath } from '../resource'
import type { PipelineStage } from '@/modules/pipelines/types'
import type { Deal } from '../types'
import { daysInStage } from './board-state'

export interface DealCardContentProps {
  deal: Deal
  formatting: RecordFormatting
  stages?: PipelineStage[]
  onMove?: (dealId: string, stageId: string) => void
  onMovePipeline?: (dealId: string) => void
  overlay?: boolean
}

export const DealCardContent = memo(function DealCardContent({
  deal,
  formatting,
  stages,
  onMove,
  onMovePipeline,
  overlay = false,
}: DealCardContentProps) {
  const { t } = useTranslation('deals')
  const location = useLocation()
  const { t: tp } = useTranslation('deals', { keyPrefix: 'pipeline' })
  const days = daysInStage(deal.stage_entered_at)
  const toneContext = useMemo(() => ({ deal }), [deal])
  const tone = useFilters('deals.tone', 'default', toneContext)
  const relation = deal.company?.name ?? deal.contact?.name
  const RelationIcon = deal.company ? Building2 : User
  const targets = stages?.filter((stage) => stage.id !== deal.stage_id) ?? []

  return (
    <div
      className={cn(
        'grid gap-2 rounded-md border bg-card p-3 text-sm shadow-xs',
        tone === 'warning' && 'border-l-4 border-l-warning-solid',
        overlay && 'rotate-1 shadow-lg ring-2 ring-primary/40',
      )}
    >
      <div className="flex items-start justify-between gap-2">
        <Link
          to={dealPath(deal.id)}
          state={{ returnTo: `${location.pathname}${location.search}` }}
          className="min-w-0 font-medium break-words hover:underline"
          onPointerDown={(event) => event.stopPropagation()}
        >
          {deal.title}
        </Link>
        {onMove && (targets.length > 0 || onMovePipeline) && (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="-mt-1 -mr-1 size-7 shrink-0"
                aria-label={t('card.moveMenu', { title: deal.title })}
                onPointerDown={(event) => event.stopPropagation()}
              >
                <EllipsisVertical aria-hidden="true" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuLabel>{t('card.moveTo')}</DropdownMenuLabel>
              {targets.map((stage) => (
                <DropdownMenuItem key={stage.id} onSelect={() => onMove(deal.id, stage.id)}>
                  {stage.name}
                </DropdownMenuItem>
              ))}
              {onMovePipeline && (
                <>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem onSelect={() => onMovePipeline(deal.id)}>
                    <Shuffle aria-hidden="true" />
                    {tp('move.menu')}
                  </DropdownMenuItem>
                </>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        )}
      </div>
      {deal.amount != null && (
        <span className="font-semibold tabular-nums">{formatCurrency(deal.amount, formatting, deal.currency)}</span>
      )}
      {relation && (
        <span className="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
          <RelationIcon className="size-3.5 shrink-0" aria-hidden="true" />
          <span className="truncate">{relation}</span>
        </span>
      )}
      <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground">
        {deal.owner ? (
          <span className="flex min-w-0 items-center gap-1.5">
            <Avatar name={deal.owner.display_name} />
            <span className="truncate">{deal.owner.display_name}</span>
          </span>
        ) : (
          <span>{t('card.noOwner')}</span>
        )}
        <span className="flex shrink-0 items-center gap-1.5">
          <Slot name="deals.badges" props={{ deal, placement: 'card' }} />
          <span className="flex items-center gap-1" title={t('card.daysInStageTitle')}>
            <Clock className="size-3.5" aria-hidden="true" />
            {t('card.daysInStage', { count: days })}
          </span>
        </span>
      </div>
    </div>
  )
})

export interface DealCardProps {
  deal: Deal
  formatting: RecordFormatting
  stages: PipelineStage[]
  disableDrag?: boolean
  draggable: boolean
  onMove: (dealId: string, stageId: string) => void
  onMovePipeline?: (dealId: string) => void
}

export const DealCard = memo(function DealCard({
  deal,
  formatting,
  stages,
  draggable,
  disableDrag,
  onMove,
  onMovePipeline,
}: DealCardProps) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: deal.id,
    data: { stageId: deal.stage_id },
    disabled: !draggable || disableDrag,
  })
  const { role: _role, tabIndex: _tabIndex, 'aria-describedby': _describedBy, ...dragAttributes } = attributes

  return (
    <li
      ref={setNodeRef}
      className={cn('list-none touch-manipulation', isDragging && 'opacity-40')}
      data-deal-id={deal.id}
      {...dragAttributes}
      {...listeners}
    >
      <DealCardContent
        deal={deal}
        formatting={formatting}
        stages={stages}
        onMove={draggable ? onMove : undefined}
        onMovePipeline={draggable ? onMovePipeline : undefined}
      />
    </li>
  )
})
