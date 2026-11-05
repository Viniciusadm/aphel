import { useMutation, useQueryClient, type QueryKey } from '@tanstack/react-query'
import { notifyError } from '@/components/feedback/notify'
import {isApprovalGateError} from '@/sdk'
import { recordsKeys } from '@/sdk'
import { dealsApi } from '../api'
import { requirementFieldsFromError } from '../requirements'
import { DEAL_RECORD_TYPE } from '../resource'
import type { Board, Deal, StageMove } from '../types'
import { moveDealOnBoard, replaceDealOnBoard } from './board-state'

interface MoveContext {
  previous?: Board
}

export function useMoveDeal(boardKey: QueryKey) {
  const queryClient = useQueryClient()
  return useMutation<Deal, unknown, StageMove, MoveContext>({
    mutationFn: (move) =>
      dealsApi.move(move.dealId, move.toStageId, move.lostReasonId, move.lostReasonNote, move.fields),
    onMutate: async (move) => {
      await queryClient.cancelQueries({ queryKey: boardKey })
      const previous = queryClient.getQueryData<Board>(boardKey)
      if (previous) {
        queryClient.setQueryData<Board>(boardKey, moveDealOnBoard(previous, move.dealId, move.toStageId))
      }
      return { previous }
    },
    onError: (error, _move, context) => {
      if (context?.previous) queryClient.setQueryData<Board>(boardKey, context.previous)
      if (!requirementFieldsFromError(error) && !isApprovalGateError(error)) notifyError(error)
    },
    onSuccess: (deal) => {
      queryClient.setQueryData<Board>(boardKey, (board) => (board ? replaceDealOnBoard(board, deal) : board))
      queryClient.setQueryData(recordsKeys.detail(DEAL_RECORD_TYPE, deal.id), deal)
      void queryClient.invalidateQueries({ queryKey: recordsKeys.list(DEAL_RECORD_TYPE) })
    },
  })
}
