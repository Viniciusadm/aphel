import type { UseQueryResult } from '@tanstack/react-query'
import { recordsApi, recordsKeys } from '@/features/records/api'

import { recordDetailApi } from '@/features/records/detail/api'
import { requireRecordResource } from '@/features/records/resources'
import type { RecordItem } from '@/features/records/types'
import { useOfflineQuery } from './cache'

const FIELD_SCHEMA_STALE_TIME = 60_000

export function useOfflineRecord(recordType: string, id: string): UseQueryResult<RecordItem> {
  const resource = requireRecordResource(recordType)
  return useOfflineQuery({
    cacheKey: `record:${recordType}:${id}`,
    queryKey: recordsKeys.detail(recordType, id),
    queryFn: () => recordDetailApi.get(resource, id),
  }).query
}

export function useOfflineFieldSchema(recordType: string) {
  return useOfflineQuery({
    cacheKey: `schema:${recordType}`,
    queryKey: recordsKeys.schema(recordType),
    queryFn: () => recordsApi.schema(recordType),
    staleTime: FIELD_SCHEMA_STALE_TIME,
  }).query
}
