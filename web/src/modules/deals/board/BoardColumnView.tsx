import { useDroppable } from '@dnd-kit/core'
import { memo } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { formatCurrency, type RecordFormatting } from '@/sdk'
import { cn } from '@/lib/utils'
import type { PipelineStage } from '@/modules/workflow/types'
import type { BoardColumn } from '../types'
import { DealCard } from './DealCard'

const KIND_ACCENT: Record<string, string> = {
  open: 'bg-primary',
  won: 'bg-success',
  lost: 'bg-destructive',
}

export interface BoardColumnViewProps {
  column: BoardColumn
  stages: PipelineStage[]
  formatting: RecordFormatting
  compact?: boolean
  disableDrag?: boolean
  draggable: boolean
  loadingMore: boolean
  onMove: (dealId: string, stageId: string) => void
  onMovePipeline?: (dealId: string) => void
  onLoadMore: (stageId: string) => void
}

export const BoardColumnView = memo(function BoardColumnView({
  column,
  stages,
  formatting,
  draggable,
  disableDrag,
  compact,
  loadingMore,
  onMove,
  onMovePipeline,
  onLoadMore,
}: BoardColumnViewProps) {
  const { t } = useTranslation('deals')
  const { setNodeRef, isOver } = useDroppable({ id: column.stage.id, data: { kind: column.stage.kind } })
  const headingId = `board-column-${column.stage.id}`

  return (
    <section
      aria-labelledby={headingId}
      className={cn(
        'flex shrink-0 snap-start flex-col rounded-lg border bg-muted/40 transition-colors',
        compact ? 'w-full' : 'w-72',
        isOver && 'border-primary bg-primary/5',
      )}
    >
      <header className="grid gap-1 border-b px-3 py-2">
        <div className="flex items-center gap-2">
          <span className={cn('size-2 rounded-full', KIND_ACCENT[column.stage.kind])} aria-hidden="true" />
          <h2 id={headingId} className="min-w-0 flex-1 truncate text-sm font-semibold">
            {column.stage.name}
          </h2>
          <Badge variant="secondary" aria-label={t('board.count', { count: column.count })}>
            {column.count}
          </Badge>
        </div>
        {column.total_amount != null && column.total_amount !== '' ? (
          <p className="text-xs text-muted-foreground tabular-nums" aria-label={t('board.totalLabel')}>
            {formatCurrency(column.total_amount, formatting)}
          </p>
        ) : null}
      </header>
      <ul ref={setNodeRef} className={cn("scroll-smooth-thin flex flex-1 flex-col overflow-y-auto", compact ? "gap-0" : "min-h-24 gap-2 p-2")}>
        {column.items.map((deal) => (
          <DealCard
            key={deal.id}
            deal={deal}
            formatting={formatting}
            stages={stages}
            draggable={draggable}
            disableDrag={disableDrag}
            compact={compact}
            onMove={onMove}
            onMovePipeline={onMovePipeline}
          />
        ))}
        {column.items.length === 0 && (
          <li className="list-none rounded-md border border-dashed p-4 text-center text-xs text-muted-foreground">
            {t('board.emptyColumn')}
          </li>
        )}
      </ul>
      {column.has_more && (
        <div className="border-t p-2">
          <Button
            variant="ghost"
            size="sm"
            className="w-full"
            disabled={loadingMore}
            onClick={() => onLoadMore(column.stage.id)}
          >
            {t('board.loadMore', { count: column.count - column.items.length })}
          </Button>
        </div>
      )}
    </section>
  )
})
