export interface AuditFilters {
  recordType: string
  recordId: string
  actor: string
  from: string
  to: string
}

export const EMPTY_FILTERS: AuditFilters = { recordType: '', recordId: '', actor: '', from: '', to: '' }

const SEARCH_KEYS: Record<keyof AuditFilters, string> = {
  recordType: 'record_type',
  recordId: 'record_id',
  actor: 'actor',
  from: 'from',
  to: 'to',
}

const FILTER_FIELDS = Object.keys(SEARCH_KEYS) as (keyof AuditFilters)[]

export const PAGE_PARAM = 'page'

export function filtersFromSearch(params: URLSearchParams): AuditFilters {
  return Object.fromEntries(
    FILTER_FIELDS.map((field) => [field, params.get(SEARCH_KEYS[field])?.trim() ?? '']),
  ) as unknown as AuditFilters
}

export function pageFromSearch(params: URLSearchParams): number {
  const page = Number.parseInt(params.get(PAGE_PARAM) ?? '', 10)
  return Number.isFinite(page) && page >= 1 ? page : 1
}

export function toSearch(filters: AuditFilters, page = 1): URLSearchParams {
  const params = new URLSearchParams()
  for (const field of FILTER_FIELDS) {
    const value = filters[field].trim()
    if (value) params.set(SEARCH_KEYS[field], value)
  }
  if (page > 1) params.set(PAGE_PARAM, String(page))
  return params
}

export function hasFilters(filters: AuditFilters): boolean {
  return FILTER_FIELDS.some((field) => filters[field].trim() !== '')
}

function localDayBoundary(date: string, endOfDay: boolean): string | undefined {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date)
  if (!match) return undefined
  const [, year, month, day] = match.map(Number) as [number, number, number, number]
  const instant = endOfDay
    ? new Date(year, month - 1, day, 23, 59, 59, 999)
    : new Date(year, month - 1, day, 0, 0, 0, 0)
  return Number.isNaN(instant.getTime()) ? undefined : instant.toISOString()
}

export function toApiQuery(filters: AuditFilters) {
  return {
    record_type: filters.recordType.trim() || undefined,
    record_id: filters.recordId.trim() || undefined,
    actor: filters.actor.trim() || undefined,
    from: localDayBoundary(filters.from, false),
    to: localDayBoundary(filters.to, true),
  }
}
