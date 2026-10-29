import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import { recordsKeys } from '../api'
import { requireRecordResource } from '../resources'
import type { Page, RecordItem, RecordListQuery } from '../types'
import { recordListParams } from './query'

export interface UseRecordListOptions {
  enabled?: boolean
}

export function recordListKey(recordType: string, query: RecordListQuery) {
  return [...recordsKeys.list(recordType), recordListParams(query)] as const
}

export function useRecordList(recordType: string, query: RecordListQuery, options: UseRecordListOptions = {}) {
  const resource = requireRecordResource(recordType)
  return useQuery({
    queryKey: recordListKey(recordType, query),
    queryFn: ({ signal }) =>
      api.get<Page<RecordItem>>(resource.basePath, { query: recordListParams(query), signal }),
    placeholderData: keepPreviousData,
    enabled: options.enabled ?? true,
  })
}
