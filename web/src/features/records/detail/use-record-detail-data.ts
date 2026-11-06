import { isApiError } from '@/api'
import { useOfflineFieldSchema, useOfflineRecord } from '@/pwa/offline/records'
import type { FieldSchema, RecordItem } from '../types'

export type RecordDetailData =
  | { status: 'loading' }
  | { status: 'not-found' }
  | { status: 'error'; retry: () => void }
  | { status: 'ready'; record: RecordItem; schema: FieldSchema }

export function useRecordDetailData(recordType: string, recordId: string): RecordDetailData {
  const schema = useOfflineFieldSchema(recordType)
  const record = useOfflineRecord(recordType, recordId)

  if (record.isError && isApiError(record.error) && record.error.status === 404) {
    return { status: 'not-found' }
  }

  if (record.isError || schema.isError) {
    return {
      status: 'error',
      retry: () => {
        if (record.isError) void record.refetch()
        if (schema.isError) void schema.refetch()
      },
    }
  }

  if (!record.data || !schema.data) return { status: 'loading' }

  return { status: 'ready', record: record.data, schema: schema.data }
}
