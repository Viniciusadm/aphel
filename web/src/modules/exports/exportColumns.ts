import { useMemo } from 'react'
import { activeFields, defaultColumnKeys, type FieldDefinition, type RecordViewConfig, requireRecordResource, useFieldSchema } from '@/sdk'
import {useSavedViews} from '@/sdk'
import {ALL_RECORDS_VIEW, resolveConfig, useListUrlState, viewConfig} from '@/sdk'

export function useCurrentListConfig(recordType: string): {
  config: RecordViewConfig | null
  fields: FieldDefinition[]
  search: string
} {
  const schema = useFieldSchema(recordType)
  const views = useSavedViews(recordType)
  const [urlState] = useListUrlState()
  const resource = requireRecordResource(recordType)
  return useMemo(() => {
    if (!schema.data) return { config: null, fields: [], search: urlState.search }
    const requested = urlState.view && urlState.view !== ALL_RECORDS_VIEW ? urlState.view : null
    const activeView = requested ? views.data?.find((view) => view.id === requested) : undefined
    const base = viewConfig(activeView, defaultColumnKeys(schema.data, resource.titleField))
    return {
      config: resolveConfig(urlState, base),
      fields: activeFields(schema.data),
      search: urlState.search,
    }
  }, [schema.data, views.data, urlState, resource.titleField])
}

export function orderedColumns(listColumns: string[], selected: Set<string>, fields: FieldDefinition[]): string[] {
  const fromList = listColumns.filter((key) => selected.has(key))
  const others = fields.map((field) => field.key).filter((key) => selected.has(key) && !fromList.includes(key))
  return [...fromList, ...others]
}

