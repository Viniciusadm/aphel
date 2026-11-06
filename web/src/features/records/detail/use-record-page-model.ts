import { createElement, useMemo, useState, type ReactNode } from 'react'
import { useLocation, useNavigate } from 'react-router'
import { useRecordCommands } from '@/features/command-palette/useRecordCommands'
import { useModuleRecordTabs } from '@/kernel/modules/contributions'
import type { RecordTabRenderProps } from '@/kernel/modules/define'
import { useFilters } from '@/kernel/modules/filters'
import { isModuleActive, useModuleState, usePermissionCheck } from '@/kernel/modules/registry'
import { activeFields, findField, getFieldValue } from '../fields'
import { resolveLayout, resolvedFields } from '../form/lib/layout'
import { useConditionContext, layoutRuleStates, recordRuleValues } from '../form/use-form-rules'
import { savedLayout, useFormLayout } from '../form/layout-api'
import { recordTitle, requireRecordResource } from '../resources'
import type { TimelineSource } from '../timeline/types'
import type { FieldDefinition, FieldSchema, RecordItem } from '../types'
import { useSaveRecordField } from './api'
import { useDefaultRecordTabs } from './default-tabs'
import { RecordSectionBoundary } from './RecordSectionBoundary'
import type { RecordBackLink } from './RecordHeader'
import { DEFAULT_READ_ONLY_FIELD_KEYS, resolveRecordTabs, type RecordTabsConfig } from './types'

export interface RecordPageModelOptions {
  recordType: string
  record: RecordItem
  schema: FieldSchema
  fieldKeys?: string[]
  readOnly?: boolean
  readOnlyFieldKeys?: string[]
  ownerField?: string
  actions?: ReactNode
  back?: RecordBackLink
  tabs?: RecordTabsConfig
  timeline?: TimelineSource
}

function rawValue(record: RecordItem, schema: FieldSchema, key: string) {
  const field = findField(schema, key)
  return { field, value: field ? getFieldValue(record, field) : (record[key] ?? record.custom_fields?.[key] ?? null) }
}

function visibleFields(schema: FieldSchema, fieldKeys?: string[]): FieldDefinition[] {
  if (!fieldKeys) return activeFields(schema)
  return fieldKeys.flatMap((key) => {
    const field = findField(schema, key)
    return field && field.is_active ? [field] : []
  })
}

export function useRecordPageModel({
  recordType,
  record,
  schema,
  fieldKeys,
  readOnly = false,
  readOnlyFieldKeys = DEFAULT_READ_ONLY_FIELD_KEYS,
  ownerField = 'owner_id',
  actions,
  back,
  tabs,
  timeline,
}: RecordPageModelOptions) {
  const navigate = useNavigate()
  const location = useLocation()
  const [returnTo] = useState(() => {
    const value = (location.state as { returnTo?: unknown } | null)?.returnTo
    return typeof value === 'string' && value.startsWith('/') && !value.startsWith('//') ? value : undefined
  })
  const contextualBack = back && returnTo ? { ...back, to: returnTo } : back
  const edit = () => void navigate(`/records/${encodeURIComponent(recordType)}/${encodeURIComponent(record.id)}/edit`, { state: { returnTo: `${location.pathname}${location.search}`, listReturnTo: returnTo } })
  const resource = requireRecordResource(recordType)
  const { saveField, isSaving } = useSaveRecordField(recordType, record.id)
  const defaults = useDefaultRecordTabs({ recordType, recordId: record.id, readOnly, timeline })
  const moduleTabs = useModuleRecordTabs(recordType, record.id, readOnly)
  const pageTabs = useMemo(() => [...resolveRecordTabs(tabs, defaults), ...moduleTabs], [tabs, defaults, moduleTabs])
  const slotProps = useMemo(() => ({ recordType, record, readOnly }), [recordType, record, readOnly])
  const filteredTabs = useFilters('records.detailTabs', pageTabs, slotProps)
  const current = useModuleState()
  const allowed = usePermissionCheck()
  const resolvedTabs = useMemo(() => filteredTabs
    .filter((tab) => (!tab.source || isModuleActive(tab.source.module, current)) && allowed(tab.permission))
    .map((tab) => {
      if (!tab.source) return tab
      const boundary = { recordType, recordId: record.id, source: tab.source, label: tab.label }
      return {
        ...tab,
        content: createElement(RecordSectionBoundary, boundary, tab.content),
        summary: tab.summary ? createElement(RecordSectionBoundary, boundary, tab.summary) : undefined,
      }
    }), [filteredTabs, current, allowed, recordType, record.id])
  const commandTabs = useMemo(() => resolvedTabs.map(({ key, label }) => ({ key, label })), [resolvedTabs])
  useRecordCommands({ recordType, recordId: record.id, tabs: commandTabs })
  const viewLayout = savedLayout(useFormLayout(recordType, 'view').data)
  const fields = useMemo(
    () => (viewLayout ? activeFields(schema) : visibleFields(schema, fieldKeys)),
    [schema, fieldKeys, viewLayout],
  )

  const conditions = useConditionContext(fields)
  const resolvedLayout = viewLayout ? resolveLayout(viewLayout, fields) : null
  const states = layoutRuleStates(resolvedLayout, recordRuleValues(fields, record), conditions)
  const layoutVisibleFields = resolvedLayout ? resolvedFields(resolvedLayout).map((item) => item.field).filter((field) => states[field.key]?.visible !== false) : fields
  const sectionContext = useMemo<RecordTabRenderProps>(
    () => ({ recordType, recordId: record.id, readOnly }),
    [recordType, record.id, readOnly],
  )

  return {
    recordType,
    recordId: record.id,
    record,
    schema,
    title: recordTitle(resource, record),
    subtitle: resource.subtitleField ? rawValue(record, schema, resource.subtitleField) : null,
    owner: rawValue(record, schema, ownerField),
    fields,
    viewLayout,
    visibleFields: layoutVisibleFields,
    summaryFieldKeys: resource.summaryFields ?? [],
    readOnly,
    readOnlyFieldKeys,
    back: contextualBack,
    edit,
    saveField,
    isSaving,
    tabs: resolvedTabs,
    slotProps,
    sectionContext,
    actions,
  }
}

export type RecordPageModel = ReturnType<typeof useRecordPageModel>
