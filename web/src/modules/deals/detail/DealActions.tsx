import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Archive, ArchiveRestore, ChevronDown, RotateCcw, ThumbsDown, ThumbsUp, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { api } from '@/api'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog } from '@/components/feedback'
import { notifyError, notifySuccess } from '@/components/feedback/notify'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { notifyApprovalGate } from '@/modules/approvals/components/useApprovalGate'
import { isApprovalGateError } from '@/modules/approvals/errors'
import { recordsKeys } from '@/features/records/api'

import type { RecordItem } from '@/features/records/types'
import { activePipelines, usePipelines } from '@/modules/workflow/api'
import { dealsApi, dealsKeys } from '../api'
import { LostReasonDialog, type LostReasonChoice } from '../LostReasonDialog'
import { MovePipelineDialog } from '../MovePipelineDialog'
import { requirementFieldsFromError } from '../requirements'
import { useDealMoveFlow, type DealMoveRequest } from '../useDealMoveFlow'
import { DEAL_RECORD_TYPE, DEALS_PATH } from '../resource'
import type { PipelineStage, StageKind } from '@/modules/workflow/types'
import type { Deal } from '../types'

export interface DealActionsProps {
  record: RecordItem
}

type DealAction =
  | { type: 'win' }
  | { type: 'lose'; choice: LostReasonChoice }
  | { type: 'reopen' }
  | { type: 'move'; request: DealMoveRequest }
  | { type: 'archive' }
  | { type: 'unarchive' }

function doneKey(action: DealAction): DealAction['type'] {
  if (action.type !== 'move' || action.request.pipelineId) return action.type
  if (action.request.to.kind === 'won') return 'win'
  if (action.request.to.kind === 'lost') return 'lose'
  return action.type
}

function closingStage(stages: PipelineStage[], kind: StageKind): PipelineStage | undefined {
  return stages
    .filter((stage) => stage.is_active && stage.kind === kind)
    .sort((left, right) => left.position - right.position)[0]
}

function runAction(id: string, action: DealAction): Promise<Deal> {
  switch (action.type) {
    case 'win':
      return dealsApi.win(id)
    case 'lose':
      return dealsApi.lose(id, action.choice.lostReasonId, action.choice.note)
    case 'reopen':
      return dealsApi.reopen(id)
    case 'move':
      return action.request.pipelineId
        ? dealsApi.movePipeline(id, {
            pipeline_id: action.request.pipelineId,
            stage_id: action.request.to.id,
            lost_reason_id: action.request.lostReasonId,
            lost_reason_note: action.request.lostReasonNote,
            fields: action.request.fields,
          })
        : dealsApi.move(
            id,
            action.request.to.id,
            action.request.lostReasonId,
            action.request.lostReasonNote,
            action.request.fields,
          )
    case 'archive':
      return api.post<Deal>(`/v1/deals/${encodeURIComponent(id)}/archive`)
    case 'unarchive':
      return api.post<Deal>(`/v1/deals/${encodeURIComponent(id)}/unarchive`)
  }
}

export function DealActions({ record }: DealActionsProps) {
  const { t } = useTranslation('deals')
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { hasPermission } = useSession()
  const deal = record as unknown as Deal
  const workflow = usePipelines()
  const [losing, setLosing] = useState(false)
  const [movingPipeline, setMovingPipeline] = useState(false)
  const { t: tp } = useTranslation('deals', { keyPrefix: 'pipeline' })
  const [deleting, setDeleting] = useState(false)
  const canUpdate = hasPermission('deals.update')
  const canDelete = hasPermission('deals.delete')
  const canArchive = hasPermission('deals.archive')

  const refresh = (updated?: Deal) => {
    if (updated) queryClient.setQueryData(recordsKeys.detail(DEAL_RECORD_TYPE, deal.id), updated)
    void queryClient.invalidateQueries({ queryKey: recordsKeys.detail(DEAL_RECORD_TYPE, deal.id) })
    void queryClient.invalidateQueries({ queryKey: recordsKeys.list(DEAL_RECORD_TYPE) })
    void queryClient.invalidateQueries({ queryKey: dealsKeys.boards() })
  }

  const action = useMutation({
    mutationFn: (next: DealAction) => runAction(deal.id, next),
    onSuccess: (updated, next) => {
      refresh(updated)
      notifySuccess(t(`detail.done.${doneKey(next)}`))
    },
    onError: (error, next) => {
      if (isApprovalGateError(error)) {
        if (next.type !== 'move') notifyApprovalGate(error)
        return
      }
      if (!requirementFieldsFromError(error)) notifyError(error)
    },
  })
  const flow = useDealMoveFlow((request) => action.mutateAsync({ type: 'move', request }), action.isPending)

  const remove = useMutation({
    mutationFn: () => api.delete<void>(`/v1/deals/${encodeURIComponent(deal.id)}`),
    onSuccess: () => {
      refresh()
      notifySuccess(t('detail.done.delete'))
      navigate(DEALS_PATH)
    },
  })

  const pipeline = workflow.data?.find((item) => item.id === deal.pipeline_id)
  const allStages = pipeline?.stages ?? []
  const stages = allStages.filter((stage) => stage.is_active && stage.id !== deal.stage_id)
  const currentStage = allStages.find((stage) => stage.id === deal.stage_id)
  const canMovePipeline = activePipelines(workflow.data).some((item) => item.id !== deal.pipeline_id)
  const busy = action.isPending
  const wonStage = closingStage(allStages, 'won')
  const lostStage = closingStage(allStages, 'lost')

  const win = () => {
    if (!wonStage) {
      action.mutate({ type: 'win' })
      return
    }
    flow.start(deal, { id: wonStage.id, name: wonStage.name, kind: 'won' }, { from: currentStage, target: wonStage })
  }

  const lose = () => {
    if (!lostStage) {
      setLosing(true)
      return
    }
    flow.start(deal, { id: lostStage.id, name: lostStage.name, kind: 'lost' }, { from: currentStage, target: lostStage })
  }

  return (
    <div className="flex flex-wrap items-center gap-2">
      {canUpdate && deal.status === 'open' && (
        <>
          <Button size="sm" variant="outline" disabled={busy} onClick={win}>
            <ThumbsUp aria-hidden="true" />
            {t('detail.win')}
          </Button>
          <Button size="sm" variant="outline" disabled={busy} onClick={lose}>
            <ThumbsDown aria-hidden="true" />
            {t('detail.lose')}
          </Button>
        </>
      )}
      {canUpdate && deal.status !== 'open' && (
        <Button size="sm" variant="outline" disabled={busy} onClick={() => action.mutate({ type: 'reopen' })}>
          <RotateCcw aria-hidden="true" />
          {t('detail.reopen')}
        </Button>
      )}
      {canUpdate && (stages.length > 0 || canMovePipeline) && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button size="sm" variant="outline" disabled={busy}>
              {t('detail.moveTo')}
              <ChevronDown aria-hidden="true" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuLabel>{t('card.moveTo')}</DropdownMenuLabel>
            {stages.map((stage) => (
              <DropdownMenuItem
                key={stage.id}
                onSelect={() =>
                  flow.start(
                    deal,
                    { id: stage.id, name: stage.name, kind: stage.kind },
                    { from: currentStage, target: stage },
                  )
                }
              >
                {stage.name}
              </DropdownMenuItem>
            ))}
            {canMovePipeline && (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuItem onSelect={() => setMovingPipeline(true)}>{tp('move.menu')}</DropdownMenuItem>
              </>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      )}
      {canArchive && (
        <Button
          size="sm"
          variant="ghost"
          disabled={busy}
          onClick={() => action.mutate({ type: deal.archived_at ? 'unarchive' : 'archive' })}
        >
          {deal.archived_at ? <ArchiveRestore aria-hidden="true" /> : <Archive aria-hidden="true" />}
          {deal.archived_at ? t('detail.unarchive') : t('detail.archive')}
        </Button>
      )}
      {canDelete && (
        <Button size="sm" variant="ghost" className="text-destructive" onClick={() => setDeleting(true)}>
          <Trash2 aria-hidden="true" />
          {t('detail.delete')}
        </Button>
      )}
      <LostReasonDialog
        open={losing}
        dealTitle={deal.title}
        pending={busy}
        onOpenChange={setLosing}
        onConfirm={(choice) => {
          setLosing(false)
          action.mutate({ type: 'lose', choice })
        }}
      />
      {flow.dialogs}
      <MovePipelineDialog
        open={movingPipeline}
        deal={deal}
        pending={busy}
        onOpenChange={setMovingPipeline}
        onConfirm={(choice) => {
          setMovingPipeline(false)
          const target = workflow.data
            ?.find((item) => item.id === choice.pipelineId)
            ?.stages.find((stage) => stage.id === choice.stageId)
          flow.start(
            deal,
            { id: choice.stageId, name: choice.stageName, kind: choice.stageKind },
            { from: currentStage, target, pipelineId: choice.pipelineId },
          )
        }}
      />
      <ConfirmDialog
        open={deleting}
        onOpenChange={setDeleting}
        title={t('detail.deleteTitle')}
        description={t('detail.deleteDescription', { title: deal.title })}
        confirmLabel={t('detail.delete')}
        variant="destructive"
        pending={remove.isPending}
        onConfirm={() => remove.mutateAsync()}
      />
    </div>
  )
}
