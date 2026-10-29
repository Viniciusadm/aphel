import { parseLayout, type ListLayout } from '../calendar/layout'
import { DEFAULT_PER_PAGE, MAX_PER_PAGE, parseSortParam, sortToParam } from '../table/query'
import {
  isFilterCondition,
  isFilterGroup,
  type FilterCondition,
  type FilterNode,
  type FilterOperator,
  type RecordViewConfig,
  type SavedView,
  type SortSpec,
  type TableDensity,
} from '../types'

export const ALL_RECORDS_VIEW = 'all'
export const DEFAULT_DENSITY: TableDensity = 'comfortable'

export const LIST_PARAMS = {
  view: 'view',
  filter: 'filter',
  sort: 'sort',
  columns: 'columns',
  density: 'density',
  search: 'search',
  page: 'page',
  perPage: 'per_page',
  layout: 'layout',
  calendarField: 'calendar_field',
} as const

export interface ListUrlState {
  view: string | null
  filter?: FilterNode | null
  sort?: SortSpec[]
  columns?: string[]
  density?: TableDensity
  search: string
  page: number
  perPage: number
  layout?: ListLayout
  calendarField?: string
}

export const EMPTY_LIST_URL_STATE: ListUrlState = { view: null, search: '', page: 1, perPage: DEFAULT_PER_PAGE }

function isFilterNode(value: unknown, depth = 1): value is FilterNode {
  if (typeof value !== 'object' || value === null || depth > 8) return false
  const node = value as FilterNode
  if (isFilterGroup(node)) {
    return (
      (node.op === 'and' || node.op === 'or') &&
      Array.isArray(node.conditions) &&
      node.conditions.every((child) => isFilterNode(child, depth + 1))
    )
  }
  return isFilterCondition(node) && typeof node.field === 'string' && typeof node.operator === 'string'
}

export function parseFilterParam(raw: string | null): FilterNode | null | undefined {
  if (raw === null) return undefined
  if (raw.trim() === '') return null
  try {
    const parsed: unknown = JSON.parse(raw)
    if (parsed === null) return null
    return isFilterNode(parsed) ? parsed : undefined
  } catch {
    return undefined
  }
}

const BRACKET_FILTER = /^filter\[([^\][]+)\](?:\[([^\][]+)\])?$/

function bracketFilterKeys(params: URLSearchParams): string[] {
  return [...params.keys()].filter((key) => BRACKET_FILTER.test(key))
}

export function parseBracketFilters(params: URLSearchParams): FilterNode | undefined {
  const conditions: FilterCondition[] = []
  for (const key of bracketFilterKeys(params)) {
    const match = BRACKET_FILTER.exec(key)
    if (!match) continue
    const [, field, operator = 'eq'] = match
    const value = params.get(key) ?? ''
    conditions.push({ field, operator: operator as FilterOperator, value: value === '' ? null : value })
  }
  if (conditions.length === 0) return undefined
  return conditions.length === 1 ? conditions[0] : { op: 'and', conditions }
}

function parsePositiveInt(raw: string | null, fallback: number, max = Number.MAX_SAFE_INTEGER): number {
  if (!raw) return fallback
  const value = Number(raw)
  return Number.isInteger(value) && value >= 1 ? Math.min(value, max) : fallback
}

function parseDensity(raw: string | null): TableDensity | undefined {
  return raw === 'compact' || raw === 'comfortable' ? raw : undefined
}

export function parseListSearch(params: URLSearchParams): ListUrlState {
  const state: ListUrlState = {
    view: params.get(LIST_PARAMS.view) || null,
    search: params.get(LIST_PARAMS.search) ?? '',
    page: parsePositiveInt(params.get(LIST_PARAMS.page), 1),
    perPage: parsePositiveInt(params.get(LIST_PARAMS.perPage), DEFAULT_PER_PAGE, MAX_PER_PAGE),
  }
  const explicit = parseFilterParam(params.get(LIST_PARAMS.filter))
  const filter = explicit === undefined ? parseBracketFilters(params) : explicit
  if (filter !== undefined) state.filter = filter
  if (params.has(LIST_PARAMS.sort)) state.sort = parseSortParam(params.get(LIST_PARAMS.sort))
  if (params.has(LIST_PARAMS.columns)) {
    state.columns = (params.get(LIST_PARAMS.columns) ?? '')
      .split(',')
      .map((key) => key.trim())
      .filter(Boolean)
  }
  const density = parseDensity(params.get(LIST_PARAMS.density))
  if (density) state.density = density
  const layout = parseLayout(params.get(LIST_PARAMS.layout))
  if (layout) state.layout = layout
  const calendarField = params.get(LIST_PARAMS.calendarField)
  if (calendarField) state.calendarField = calendarField
  return state
}

export function buildListSearch(state: ListUrlState, base?: URLSearchParams): URLSearchParams {
  const params = new URLSearchParams(base)
  for (const key of Object.values(LIST_PARAMS)) params.delete(key)
  for (const key of bracketFilterKeys(params)) params.delete(key)
  if (state.view) params.set(LIST_PARAMS.view, state.view)
  if (state.filter !== undefined) params.set(LIST_PARAMS.filter, state.filter ? JSON.stringify(state.filter) : '')
  if (state.sort !== undefined) params.set(LIST_PARAMS.sort, sortToParam(state.sort))
  if (state.columns !== undefined) params.set(LIST_PARAMS.columns, state.columns.join(','))
  if (state.density !== undefined) params.set(LIST_PARAMS.density, state.density)
  if (state.search.trim()) params.set(LIST_PARAMS.search, state.search)
  if (state.page > 1) params.set(LIST_PARAMS.page, String(state.page))
  if (state.perPage !== DEFAULT_PER_PAGE) params.set(LIST_PARAMS.perPage, String(state.perPage))
  if (state.layout) params.set(LIST_PARAMS.layout, state.layout)
  if (state.calendarField) params.set(LIST_PARAMS.calendarField, state.calendarField)
  return params
}

export function hasExplicitQuery(state: ListUrlState): boolean {
  return (
    state.filter !== undefined ||
    state.sort !== undefined ||
    state.columns !== undefined ||
    state.density !== undefined ||
    state.layout !== undefined ||
    state.search.trim() !== ''
  )
}

export function viewDensity(view: SavedView | undefined): TableDensity | undefined {
  const density = (view?.settings as { density?: unknown } | undefined)?.density
  return density === 'compact' || density === 'comfortable' ? density : undefined
}

export function viewConfig(view: SavedView | undefined, defaultColumns: string[]): RecordViewConfig {
  return {
    filter: view?.filter ?? null,
    sort: view?.sort ?? [],
    columns: view && view.columns.length > 0 ? view.columns : defaultColumns,
    density: viewDensity(view) ?? DEFAULT_DENSITY,
  }
}

export function resolveConfig(state: ListUrlState, base: RecordViewConfig): RecordViewConfig {
  return {
    filter: state.filter !== undefined ? state.filter : base.filter,
    sort: state.sort ?? base.sort,
    columns: state.columns && state.columns.length > 0 ? state.columns : base.columns,
    density: state.density ?? base.density,
  }
}

function normalizedSort(sort: SortSpec[]): string {
  return sortToParam(sort.map((spec) => ({ field: spec.field, direction: spec.direction ?? 'asc' })))
}

export function sameConfig(left: RecordViewConfig, right: RecordViewConfig): boolean {
  return (
    JSON.stringify(left.filter ?? null) === JSON.stringify(right.filter ?? null) &&
    normalizedSort(left.sort) === normalizedSort(right.sort) &&
    left.columns.join(',') === right.columns.join(',') &&
    left.density === right.density
  )
}

export function pickDefaultView(views: SavedView[]): SavedView | undefined {
  return views.find((view) => view.is_user_default) ?? views.find((view) => view.is_default)
}
