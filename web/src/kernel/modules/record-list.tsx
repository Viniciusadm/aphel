import { Fragment, useMemo, type ReactNode } from 'react'
import type { FieldDefinition, FieldSchema, RecordItem } from '@/features/records/types'
import type { RecordListExtension, RecordListExtensionResult } from './define'
import { matchesRecordType } from './record-types'
import { contributions, isModuleActive, useModuleState, usePermissionCheck } from './registry'
import { useStableList } from './stable-list'

export interface RecordListExtensions {
  filterSchema: FieldSchema
  titleAddon?: (record: RecordItem) => ReactNode
}

type TitleAddon = NonNullable<RecordListExtensionResult['titleAddon']>

function useExtensionResults(recordType: string, schema: FieldSchema, records: RecordItem[]): RecordListExtensionResult[] {
  const current = useModuleState()
  const allowed = usePermissionCheck()
  const results: RecordListExtensionResult[] = []
  for (const { module, value } of contributions<RecordListExtension>('recordList')) {
    const enabled =
      isModuleActive(module, current) && matchesRecordType(value.recordTypes, recordType) && allowed(value.permission)
    const result = value.use({ recordType, schema, records, enabled })
    if (enabled) results.push(result)
  }
  return results
}

export function useRecordListExtensions(recordType: string, schema: FieldSchema, records: RecordItem[]): RecordListExtensions {
  const results = useExtensionResults(recordType, schema, records)
  const filterFields = useStableList<FieldDefinition[]>(results.flatMap((result) => (result.filterFields ? [result.filterFields] : [])))
  const addons = useStableList<TitleAddon>(results.flatMap((result) => (result.titleAddon ? [result.titleAddon] : [])))

  const filterSchema = useMemo(() => {
    const known = new Set(schema.fields.map((field) => field.key))
    const extra = filterFields.flat().filter((field) => {
      if (known.has(field.key)) return false
      known.add(field.key)
      return true
    })
    return extra.length > 0 ? { ...schema, fields: [...schema.fields, ...extra] } : schema
  }, [schema, filterFields])

  const titleAddon = useMemo(() => {
    if (addons.length === 0) return undefined
    return (record: RecordItem) => addons.map((addon, index) => <Fragment key={index}>{addon(record)}</Fragment>)
  }, [addons])

  return { filterSchema, titleAddon }
}
