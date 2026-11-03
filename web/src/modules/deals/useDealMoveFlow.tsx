import { useCallback, useState, type ReactNode } from 'react'
import { useApprovalGate } from '@/modules/approvals/components/useApprovalGate'
import { LostReasonDialog } from './LostReasonDialog'
import { isStageRequirementsInaccessible, missingStageFields, requirementFieldsFromError } from './requirements'
import { StageRequirementsDialog } from './StageRequirementsDialog'
import type { PipelineStage, StageKind } from '@/modules/workflow/types'
import type { Deal } from './types'

export interface MoveTarget {
  id: string
  name: string
  kind: StageKind
}

export interface DealMoveRequest {
  deal: Deal
  to: MoveTarget
  pipelineId?: string
  lostReasonId?: string
  lostReasonNote?: string
  fields?: Record<string, unknown>
}

interface FlowState {
  request: DealMoveRequest
  step: 'requirements' | 'lost'
  missing: string[]
  inaccessible?: boolean
}

export interface DealMoveFlow {
  start: (
    deal: Deal,
    to: MoveTarget,
    options?: { from?: PipelineStage; target?: PipelineStage; pipelineId?: string },
  ) => void
  dialogs: ReactNode
}

export function useDealMoveFlow(
  execute: (request: DealMoveRequest) => Promise<unknown>,
  pending = false,
  onSettled?: (deal: Deal) => void,
): DealMoveFlow {
  const [state, setState] = useState<FlowState | null>(null)
  const { handle: handleApproval, dialog: approvalDialog } = useApprovalGate()

  const run = useCallback(
    async (request: DealMoveRequest) => {
      try {
        await execute(request)
      } catch (error) {
        if (handleApproval(error)) return
        const missing = requirementFieldsFromError(error)
        if (missing) {
          setState({ request, step: 'requirements', missing, inaccessible: isStageRequirementsInaccessible(error) })
          return
        }
      }
      onSettled?.(request.deal)
    },
    [execute, handleApproval, onSettled],
  )

  const proceed = useCallback(
    (request: DealMoveRequest) => {
      if (request.to.kind === 'lost' && !request.lostReasonId) {
        setState({ request, step: 'lost', missing: [] })
        return
      }
      setState(null)
      void run(request)
    },
    [run],
  )

  const start = useCallback<DealMoveFlow['start']>(
    (deal, to, options = {}) => {
      const request: DealMoveRequest = { deal, to, pipelineId: options.pipelineId }
      const missing = options.target ? missingStageFields(deal, options.from, options.target) : []
      if (missing.length > 0) {
        setState({ request, step: 'requirements', missing })
        return
      }
      proceed(request)
    },
    [proceed],
  )

  const close = (open: boolean) => {
    if (open || !state) return
    const { deal } = state.request
    setState(null)
    onSettled?.(deal)
  }

  const dialogs = (
    <>
      <StageRequirementsDialog
        open={state?.step === 'requirements'}
        deal={state?.request.deal}
        stageName={state?.request.to.name}
        fields={state?.missing ?? []}
        inaccessible={state?.inaccessible ?? false}
        pending={pending}
        onOpenChange={close}
        onConfirm={(values) => {
          if (!state) return
          proceed({ ...state.request, fields: { ...state.request.fields, ...values } })
        }}
      />
      <LostReasonDialog
        open={state?.step === 'lost'}
        dealTitle={state?.request.deal.title}
        pending={pending}
        onOpenChange={close}
        onConfirm={(choice) => {
          if (!state) return
          proceed({ ...state.request, lostReasonId: choice.lostReasonId, lostReasonNote: choice.note })
        }}
      />
      {approvalDialog}
    </>
  )

  return { start, dialogs }
}
