import type { components } from '@/api'

type Schemas = components['schemas']

export type Deal = Schemas['DealResponse']
export type DealStatus = Schemas['DealStatus']
export type DealsPage = Schemas['Page_DealResponse']
export type BoardColumn = Omit<Schemas['BoardColumnResponse'], 'total_amount'> & { total_amount?: string | null }
export type Board = Omit<Schemas['BoardResponse'], 'columns'> & { columns: BoardColumn[] }

export interface StageMove {
  dealId: string
  toStageId: string
  lostReasonId?: string
  lostReasonNote?: string
  fields?: Record<string, unknown>
}
