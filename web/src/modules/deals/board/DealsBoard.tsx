import { DndContext, DragOverlay, closestCorners, type DragEndEvent, type DragStartEvent } from '@dnd-kit/core'
import { type PipelineStage, activePipelines, formatCurrency, recordsKeys, useBoardSensors, useMediaQuery, usePipelines, useRecordFormatting } from '@/sdk'
import { Button } from '@/components/ui/button'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { KanbanSquare } from 'lucide-react'
import { useCallback, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback/notify'
import { EmptyState } from '@/components/ui/empty-state'
import { isApprovalGateError } from '@/modules/approvals/errors'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { BOARD_PER_COLUMN, dealsApi, dealsKeys, type BoardRequest } from '../api'
import { MovePipelineDialog, type PipelineMoveChoice } from '../MovePipelineDialog'
import { requirementFieldsFromError } from '../requirements'
import { DEAL_RECORD_TYPE } from '../resource'
import type { Board, Deal } from '../types'
import { useDealMoveFlow, type DealMoveRequest } from '../useDealMoveFlow'
import { appendColumnPage, findColumn, locateDeal } from './board-state'
import { BoardColumnView } from './BoardColumnView'
import { DealCardContent } from './DealCard'
import { useMoveDeal } from './useMoveDeal'

export interface DealsBoardProps {
  request: BoardRequest
  canMove: boolean
}

const EMPTY_STAGES: PipelineStage[] = []

function BoardSkeleton() {
  return (
    <div className="flex gap-3 overflow-hidden" aria-busy="true">
      {[0, 1, 2, 3].map((index) => (
        <Skeleton key={index} className="h-96 w-72 shrink-0" />
      ))}
    </div>
  )
}

export function DealsBoard({ request, canMove }: DealsBoardProps) {
  const { t } = useTranslation('deals')
  const queryClient = useQueryClient()
  const mobile = useMediaQuery('(max-width: 767px)')
  const [mobileStage, setMobileStage] = useState('')
  const [mobileBoard, setMobileBoard] = useState(false)
  const formatting = useRecordFormatting()
  const requestKey = JSON.stringify(dealsKeys.board(request))
  const boardKey = useMemo(() => JSON.parse(requestKey) as unknown[], [requestKey])
  const board = useQuery({ queryKey: boardKey, queryFn: () => dealsApi.board(request) })
  const move = useMoveDeal(boardKey)
  const { mutateAsync } = move
  const [movingPipeline, setMovingPipeline] = useState<Deal | null>(null)
  const [activeId, setActiveId] = useState<string | null>(null)
  const [loadingStage, setLoadingStage] = useState<string | null>(null)

  const columns = board.data?.columns
  const workflow = usePipelines()
  const pipelineId = board.data?.pipeline.id
  const allStages = useMemo(
    () => workflow.data?.find((pipeline) => pipeline.id === pipelineId)?.stages ?? EMPTY_STAGES,
    [workflow.data, pipelineId],
  )
  const stages = useMemo(() => allStages.filter((stage) => stage.is_active), [allStages])
  const canMovePipeline = activePipelines(workflow.data).length > 1

  const execute = useCallback(
    async (request: DealMoveRequest) => {
      if (request.pipelineId) {
        const updated = await dealsApi.movePipeline(request.deal.id, {
          pipeline_id: request.pipelineId,
          stage_id: request.to.id,
          lost_reason_id: request.lostReasonId,
          lost_reason_note: request.lostReasonNote,
          fields: request.fields,
        })
        queryClient.setQueryData(recordsKeys.detail(DEAL_RECORD_TYPE, updated.id), updated)
        void queryClient.invalidateQueries({ queryKey: dealsKeys.boards() })
        void queryClient.invalidateQueries({ queryKey: recordsKeys.list(DEAL_RECORD_TYPE) })
        return updated
      }
      return mutateAsync({
        dealId: request.deal.id,
        toStageId: request.to.id,
        lostReasonId: request.lostReasonId,
        lostReasonNote: request.lostReasonNote,
        fields: request.fields,
      })
    },
    [mutateAsync, queryClient],
  )
  const focusDeal = useCallback((deal: Deal) => {
    const focus = () => {
      const card = document.querySelector<HTMLElement>(`[data-deal-id="${CSS.escape(deal.id)}"] a`)
      card?.focus()
      card?.scrollIntoView({ block: 'nearest', inline: 'nearest' })
    }
    window.setTimeout(() => window.requestAnimationFrame(focus), 0)
  }, [])
  const flow = useDealMoveFlow(
    async (request) => {
      try {
        await execute(request)
      } catch (error) {
        if (request.pipelineId && !requirementFieldsFromError(error) && !isApprovalGateError(error)) notifyError(error)
        throw error
      }
    },
    move.isPending,
    focusDeal,
  )
  const { start } = flow

  const sensors = useBoardSensors()

  const requestMove = useCallback(
    (dealId: string, stageId: string) => {
      const current = queryClient.getQueryData<Board>(boardKey)
      if (!current) return
      const source = locateDeal(current, dealId)
      const target = findColumn(current, stageId)
      if (!source || !target || source.deal.stage_id === stageId) return
      start(
        source.deal,
        { id: target.stage.id, name: target.stage.name, kind: target.stage.kind },
        {
          from: allStages.find((stage) => stage.id === source.deal.stage_id),
          target: allStages.find((stage) => stage.id === stageId) ?? target.stage,
        },
      )
    },
    [queryClient, boardKey, start, allStages],
  )

  const requestPipelineMove = useCallback(
    (dealId: string) => {
      const current = queryClient.getQueryData<Board>(boardKey)
      const source = current && locateDeal(current, dealId)
      if (source) setMovingPipeline(source.deal)
    },
    [queryClient, boardKey],
  )

  const loadMore = useCallback(
    async (stageId: string) => {
      const current = queryClient.getQueryData<Board>(boardKey)
      const column = current && findColumn(current, stageId)
      if (!column) return
      setLoadingStage(stageId)
      try {
        const page = Math.floor(column.items.length / BOARD_PER_COLUMN) + 1
        const result = await dealsApi.boardColumn(request, stageId, page)
        queryClient.setQueryData<Board>(boardKey, (existing) =>
          existing ? appendColumnPage(existing, result) : existing,
        )
      } catch (error) {
        notifyError(error)
      } finally {
        setLoadingStage(null)
      }
    },
    [queryClient, boardKey, request],
  )

  const handleDragStart = useCallback((event: DragStartEvent) => setActiveId(String(event.active.id)), [])
  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      setActiveId(null)
      if (event.over) requestMove(String(event.active.id), String(event.over.id))
    },
    [requestMove],
  )
  const handleDragCancel = useCallback(() => setActiveId(null), [])

  const confirmPipelineMove = (deal: Deal, choice: PipelineMoveChoice) => {
    setMovingPipeline(null)
    const target = workflow.data
      ?.find((pipeline) => pipeline.id === choice.pipelineId)
      ?.stages.find((stage) => stage.id === choice.stageId)
    start(
      deal,
      { id: choice.stageId, name: choice.stageName, kind: choice.stageKind },
      { from: allStages.find((stage) => stage.id === deal.stage_id), target, pipelineId: choice.pipelineId },
    )
  }

  if (board.isPending) return <BoardSkeleton />
  if (board.isError || !columns) {
    return <ErrorState title={t('board.loadError')} onRetry={() => void board.refetch()} />
  }
  if (columns.length === 0) {
    return <EmptyState icon={KanbanSquare} title={t('board.noStages')} description={t('board.noStagesDescription')} />
  }

  const active = activeId && board.data ? locateDeal(board.data, activeId)?.deal : undefined

  return (
    <>
      {mobile && <div className="flex justify-end"><Button variant="ghost" size="sm" aria-pressed={mobileBoard} onClick={() => setMobileBoard((current) => !current)}>{mobileBoard ? t('board.groupedList') : t('list.board')}</Button></div>}
      {mobile && <nav aria-label={t('board.stageSelect')} className="grid grid-cols-2 gap-2">
        {columns.map((column) => <Button key={column.stage.id} variant={mobileStage === column.stage.id ? 'secondary' : 'outline'} className="h-auto justify-between whitespace-normal px-3 py-2 text-left" aria-pressed={mobileStage === column.stage.id} onClick={() => setMobileStage((current) => current === column.stage.id ? '' : column.stage.id)}>
          <span className="grid min-w-0 gap-0.5"><span>{column.stage.name}</span>{column.total_amount != null && <span className="text-xs font-normal text-muted-foreground">{formatCurrency(column.total_amount, formatting)}</span>}</span><span className="tabular-nums">{column.count}</span>
        </Button>)}
      </nav>}
      <DndContext
        sensors={sensors}
        collisionDetection={closestCorners}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
        onDragCancel={handleDragCancel}
        accessibility={{
          screenReaderInstructions: { draggable: t('board.dragInstructions') },
        }}
      >
        <div
          role="region"
          aria-label={t('board.label', { pipeline: board.data.pipeline.name })}
          className={mobile && !mobileBoard ? "grid min-w-0 gap-3" : "scroll-smooth-thin flex min-h-0 flex-1 gap-3 overflow-x-auto overscroll-x-contain pb-3"}
        >
          {(mobile && !mobileBoard && mobileStage && columns.some((column) => column.stage.id === mobileStage) ? columns.filter((column) => column.stage.id === mobileStage) : columns).map((column) => (
            <BoardColumnView
              key={column.stage.id}
              column={column}
              stages={stages}
              formatting={formatting}
              draggable={canMove}
              disableDrag={mobile}
              compact={mobile && !mobileBoard}
              loadingMore={loadingStage === column.stage.id}
              onMove={requestMove}
              onMovePipeline={canMovePipeline ? requestPipelineMove : undefined}
              onLoadMore={loadMore}
            />
          ))}
        </div>
        <DragOverlay dropAnimation={null}>
          {active ? <DealCardContent deal={active} formatting={formatting} overlay /> : null}
        </DragOverlay>
      </DndContext>
      {flow.dialogs}
      <MovePipelineDialog
        open={movingPipeline !== null}
        deal={movingPipeline ?? undefined}
        onOpenChange={(open) => {
          if (!open) setMovingPipeline(null)
        }}
        onConfirm={(choice) => movingPipeline && confirmPipelineMove(movingPipeline, choice)}
      />
    </>
  )
}
