import type { PipelineStage, StageKind } from '@/modules/workflow/types'
import type { Board, BoardColumn, Deal, DealStatus } from '../types'

export interface DealLocation {
  columnIndex: number
  itemIndex: number
  deal: Deal
}

const STATUS_BY_KIND: Record<StageKind, DealStatus> = {
  open: 'open',
  won: 'won',
  lost: 'lost',
}

export function statusForStage(kind: StageKind): DealStatus {
  return STATUS_BY_KIND[kind]
}

export function locateDeal(board: Board, dealId: string): DealLocation | null {
  for (let columnIndex = 0; columnIndex < board.columns.length; columnIndex += 1) {
    const itemIndex = board.columns[columnIndex].items.findIndex((deal) => deal.id === dealId)
    if (itemIndex >= 0) {
      return { columnIndex, itemIndex, deal: board.columns[columnIndex].items[itemIndex] }
    }
  }
  return null
}

export function findColumn(board: Board, stageId: string): BoardColumn | undefined {
  return board.columns.find((column) => column.stage.id === stageId)
}

export function moneyToCents(value: string | number | null | undefined): number {
  if (value === null || value === undefined || value === '') return 0
  const parsed = Number(value)
  return Number.isFinite(parsed) ? Math.round(parsed * 100) : 0
}

export function centsToMoney(cents: number): string {
  return (cents / 100).toFixed(2)
}

function amountOf(deal: Deal): number {
  return moneyToCents(deal.amount)
}

function addMoney(total: string | null | undefined, cents: number): string | null | undefined {
  if (total === null || total === undefined || total === '') return total
  return centsToMoney(moneyToCents(total) + cents)
}

export function dealInStage(deal: Deal, stage: PipelineStage, enteredAt: string): Deal {
  const status = statusForStage(stage.kind)
  return {
    ...deal,
    stage_id: stage.id,
    pipeline_id: stage.pipeline_id,
    stage: { id: stage.id, name: stage.name, kind: stage.kind, probability: stage.probability },
    status,
    stage_entered_at: enteredAt,
    closed_at: status === 'open' ? null : (deal.closed_at ?? enteredAt),
  }
}

export function moveDealOnBoard(
  board: Board,
  dealId: string,
  toStageId: string,
  enteredAt: string = new Date().toISOString(),
  toIndex = 0,
): Board {
  const source = locateDeal(board, dealId)
  const targetIndex = board.columns.findIndex((column) => column.stage.id === toStageId)
  if (!source || targetIndex < 0 || source.columnIndex === targetIndex) return board

  const moved = dealInStage(source.deal, board.columns[targetIndex].stage, enteredAt)
  const amount = amountOf(source.deal)
  const columns = board.columns.map((column, index) => {
    if (index === source.columnIndex) {
      return {
        ...column,
        count: Math.max(0, column.count - 1),
        total_amount: addMoney(column.total_amount, -amount),
        items: column.items.filter((deal) => deal.id !== dealId),
      }
    }
    if (index === targetIndex) {
      const items = [...column.items]
      items.splice(Math.min(Math.max(toIndex, 0), items.length), 0, moved)
      return {
        ...column,
        count: column.count + 1,
        total_amount: addMoney(column.total_amount, amountOf(moved)),
        items,
      }
    }
    return column
  })
  return { ...board, columns }
}

export function replaceDealOnBoard(board: Board, deal: Deal): Board {
  const location = locateDeal(board, deal.id)
  if (!location) return board
  const current = location.deal
  if (current.stage_id !== deal.stage_id) {
    const moved = moveDealOnBoard(board, deal.id, deal.stage_id, deal.stage_entered_at)
    return moved === board ? board : replaceDealOnBoard(moved, deal)
  }
  const delta = amountOf(deal) - amountOf(current)
  const columns = board.columns.map((column, index) => {
    if (index !== location.columnIndex) return column
    const items = [...column.items]
    items[location.itemIndex] = deal
    return { ...column, items, total_amount: addMoney(column.total_amount, delta) }
  })
  return { ...board, columns }
}

export function appendColumnPage(board: Board, page: Board): Board {
  const incoming = page.columns[0]
  if (!incoming) return board
  const columns = board.columns.map((column) => {
    if (column.stage.id !== incoming.stage.id) return column
    const known = new Set(column.items.map((deal) => deal.id))
    const items = [...column.items, ...incoming.items.filter((deal) => !known.has(deal.id))]
    return {
      ...column,
      count: incoming.count,
      total_amount: incoming.total_amount,
      has_more: incoming.has_more,
      items,
    }
  })
  return { ...board, columns }
}

export function daysInStage(enteredAt: string, now: Date = new Date()): number {
  const entered = new Date(enteredAt)
  if (Number.isNaN(entered.getTime())) return 0
  return Math.max(0, Math.floor((now.getTime() - entered.getTime()) / 86_400_000))
}
