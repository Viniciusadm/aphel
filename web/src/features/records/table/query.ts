import type { QueryParams } from '@/api'
import type { RecordListQuery, SortSpec } from '../types'

export const MAX_SORTS = 3
export const DEFAULT_PER_PAGE = 25
export const PER_PAGE_OPTIONS = [25, 50, 100] as const
export const MAX_PER_PAGE = 100

export function sortToParam(sort: SortSpec[] | undefined): string {
  if (!sort) return ''
  return sort
    .slice(0, MAX_SORTS)
    .map((spec) => (spec.direction === 'desc' ? `-${spec.field}` : spec.field))
    .join(',')
}

export function parseSortParam(raw: string | null | undefined): SortSpec[] {
  if (!raw) return []
  const specs: SortSpec[] = []
  for (const part of raw.split(',')) {
    const token = part.trim()
    if (!token) continue
    const descending = token.startsWith('-')
    const field = descending ? token.slice(1).trim() : token
    if (!field || specs.some((spec) => spec.field === field)) continue
    specs.push({ field, direction: descending ? 'desc' : 'asc' })
  }
  return specs.slice(0, MAX_SORTS)
}

export function recordListParams(query: RecordListQuery): QueryParams {
  const sort = sortToParam(query.sort)
  const search = query.search?.trim()
  return {
    page: query.page,
    per_page: query.perPage,
    filter: query.filter ? JSON.stringify(query.filter) : undefined,
    sort: sort || undefined,
    search: search || undefined,
    archived: query.archived,
  }
}
