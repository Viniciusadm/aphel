import { api } from '@/api'
import { type FilterNode } from '@/sdk'
import type { Board, Deal, DealsPage } from './types'

export const BOARD_PER_COLUMN = 25
export const RELATED_DEALS_PER_PAGE = 20

export interface BoardRequest {
  pipelineId?: string | null
  filter?: FilterNode | null
  search?: string
}

export const dealsKeys = {
  all: ['deals'] as const,
  boards: () => [...dealsKeys.all, 'board'] as const,
  board: (request: BoardRequest) =>
    [...dealsKeys.boards(), request.pipelineId ?? null, request.filter ?? null, request.search ?? ''] as const,
  related: (field: string, recordId: string) => [...dealsKeys.all, 'related', field, recordId] as const,
}

function boardQuery(request: BoardRequest, extra: Record<string, string | number | undefined> = {}) {
  return {
    pipeline_id: request.pipelineId ?? undefined,
    filter: request.filter ? JSON.stringify(request.filter) : undefined,
    search: request.search?.trim() || undefined,
    per_column: BOARD_PER_COLUMN,
    ...extra,
  }
}

function dealPath(id: string, action?: string) {
  const base = `/v1/deals/${encodeURIComponent(id)}`
  return action ? `${base}/${action}` : base
}

export const dealsApi = {
  board: (request: BoardRequest) => api.get<Board>('/v1/deals/board', { query: boardQuery(request) }),
  boardColumn: (request: BoardRequest, stageId: string, page: number) =>
    api.get<Board>('/v1/deals/board', { query: boardQuery(request, { stage_id: stageId, page }) }),
  related: (field: string, recordId: string) =>
    api.get<DealsPage>('/v1/deals', {
      query: {
        filter: JSON.stringify({ field, operator: 'eq', value: recordId }),
        sort: '-created_at',
        per_page: RELATED_DEALS_PER_PAGE,
      },
    }),
  move: (
    id: string,
    stageId: string,
    lostReasonId?: string,
    lostReasonNote?: string,
    fields?: Record<string, unknown>,
  ) =>
    api.post<Deal>(dealPath(id, 'move'), {
      stage_id: stageId,
      lost_reason_id: lostReasonId,
      lost_reason_note: lostReasonNote || undefined,
      fields: fields && Object.keys(fields).length > 0 ? fields : undefined,
    }),
  movePipeline: (id: string, body: MovePipelineBody) => api.post<Deal>(dealPath(id, 'move-pipeline'), body),
  win: (id: string) => api.post<Deal>(dealPath(id, 'won')),
  lose: (id: string, lostReasonId: string, lostReasonNote?: string) =>
    api.post<Deal>(dealPath(id, 'lost'), {
      lost_reason_id: lostReasonId,
      lost_reason_note: lostReasonNote || undefined,
    }),
  reopen: (id: string, stageId?: string) => api.post<Deal>(dealPath(id, 'reopen'), { stage_id: stageId }),
}

export interface MovePipelineBody {
  pipeline_id: string
  stage_id?: string
  lost_reason_id?: string
  lost_reason_note?: string
  fields?: Record<string, unknown>
}
