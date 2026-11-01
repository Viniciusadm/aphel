import { Pencil } from 'lucide-react'
import { useMemo, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { useRecordCommands } from '@/features/command-palette/useRecordCommands'
import { useMediaQuery } from '../calendar/useMediaQuery'
import { useModuleRecordTabs } from '@/kernel/modules/contributions'
import { useFilters } from '@/kernel/modules/filters'
import { Slot } from '@/kernel/modules/Slot'
import { FieldValueDisplay, UserValue } from '../display/FieldValueDisplay'
import { ShareRecordButton } from '../sharing/ShareRecordButton'
import { DisplayRecordProvider } from '../display/display-record'
import { activeFields, findField, getFieldValue } from '../fields'
import { savedLayout, useFormLayout } from '../form/layout-api'
import { RecordFormDialog } from '../form/RecordFormDialog'
import { recordTitle, requireRecordResource } from '../resources'
import type { TimelineSource } from '../timeline/types'
import type { FieldDefinition, FieldSchema, RecordItem } from '../types'
import { useSaveRecordField } from './api'
import { useDefaultRecordTabs } from './default-tabs'
import { RecordFieldsPanel } from './RecordFieldsPanel'
import { RecordHeader, type RecordBackLink } from './RecordHeader'
import { RecordMobileView } from './mobile/RecordMobileView'
import { RecordPageShell } from './RecordPageShell'
import { RecordTabs } from './RecordTabs'
import { DEFAULT_READ_ONLY_FIELD_KEYS, resolveRecordTabs, type RecordTabsConfig } from './types'

const MOBILE_QUERY = '(max-width: 767px)'
const MOBILE_SUMMARY_FIELDS = 6

export interface RecordPageLayoutProps {
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
  defaultTab?: string
  timeline?: TimelineSource
  className?: string
}

function rawValue(record: RecordItem, schema: FieldSchema, key: string) {
  const field = findField(schema, key)
  return { field, value: field ? getFieldValue(record, field) : (record[key] ?? record.custom_fields?.[key] ?? null) }
}

function hasValue(value: unknown) {
  if (value === null || value === undefined) return false
  if (typeof value === 'string') return value.trim() !== ''
  return !Array.isArray(value) || value.length > 0
}

function visibleFields(schema: FieldSchema, fieldKeys?: string[]): FieldDefinition[] {
  if (!fieldKeys) return activeFields(schema)
  return fieldKeys.flatMap((key) => {
    const field = findField(schema, key)
    return field && field.is_active ? [field] : []
  })
}

export function RecordPageLayout({
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
  defaultTab,
  timeline,
  className,
}: RecordPageLayoutProps) {
  const { t } = useTranslation('records-detail')
  const [editing, setEditing] = useState(false)
  const resource = requireRecordResource(recordType)
  const { saveField } = useSaveRecordField(recordType, record.id)
  const defaults = useDefaultRecordTabs({ recordType, recordId: record.id, readOnly, timeline })
  const moduleTabs = useModuleRecordTabs(recordType, record.id, readOnly)
  const pageTabs = useMemo(() => [...resolveRecordTabs(tabs, defaults), ...moduleTabs], [tabs, defaults, moduleTabs])
  const slotProps = useMemo(() => ({ recordType, record, readOnly }), [recordType, record, readOnly])
  const resolvedTabs = useFilters('records.detailTabs', pageTabs, slotProps)
  const commandTabs = useMemo(() => resolvedTabs.map(({ key, label }) => ({ key, label })), [resolvedTabs])
  useRecordCommands({ recordType, recordId: record.id, tabs: commandTabs })
  const viewLayout = savedLayout(useFormLayout(recordType, 'view').data)
  const fields = useMemo(
    () => (viewLayout ? activeFields(schema) : visibleFields(schema, fieldKeys)),
    [schema, fieldKeys, viewLayout],
  )

  const subtitle = resource.subtitleField ? rawValue(record, schema, resource.subtitleField) : null
  const owner = rawValue(record, schema, ownerField)
  const isMobile = useMediaQuery(MOBILE_QUERY)
  const title = recordTitle(resource, record)
  const subtitleNode =
    subtitle && hasValue(subtitle.value) ? (
      subtitle.field ? (
        <FieldValueDisplay field={subtitle.field} value={subtitle.value} />
      ) : (
        String(subtitle.value)
      )
    ) : undefined
  const ownerNode = typeof owner.value === 'string' && owner.value ? <UserValue userId={owner.value} /> : undefined

  if (isMobile) {
    return (
      <DisplayRecordProvider value={{ recordType, recordId: record.id, title }}>
        <RecordMobileView
          className={className}
          title={title}
          titleAccessory={<Slot name="record.detail.title" props={slotProps} />}
          headerSlot={<Slot name="record.detail.header" props={slotProps} />}
          subtitle={subtitleNode}
          owner={ownerNode}
          back={back}
          tabs={resolvedTabs}
          onEdit={readOnly ? undefined : () => setEditing(true)}
          actions={
            <>
              <ShareRecordButton recordType={recordType} recordId={record.id} />
              {actions}
              <Slot name="record.detail.actions" props={slotProps} />
            </>
          }
          summaryPanel={
            <RecordFieldsPanel
              fields={fields.slice(0, MOBILE_SUMMARY_FIELDS)}
              record={record}
              readOnly={readOnly}
              readOnlyFieldKeys={readOnlyFieldKeys}
              onSaveField={saveField}
            />
          }
          detailsPanel={
            <div className="grid min-w-0 gap-4">
              <RecordFieldsPanel
                title=""
                className="border-0 p-0"
                fields={fields}
                layout={viewLayout}
                record={record}
                readOnly={readOnly}
                readOnlyFieldKeys={readOnlyFieldKeys}
                onSaveField={saveField}
              >
                <Slot name="record.fields.panel" props={slotProps} />
              </RecordFieldsPanel>
              <Slot name="record.detail.sidebar" props={slotProps} />
            </div>
          }
        />
        {!readOnly && (
          <RecordFormDialog recordType={recordType} record={record} open={editing} onOpenChange={setEditing} />
        )}
      </DisplayRecordProvider>
    )
  }

  return (
    <DisplayRecordProvider value={{ recordType, recordId: record.id, title: recordTitle(resource, record) }}>
      <RecordPageShell
        className={className}
        header={
          <>
          <RecordHeader
            title={recordTitle(resource, record)}
            titleAccessory={<Slot name="record.detail.title" props={slotProps} />}
            back={back}
            actions={
              <>
                {!readOnly && (
                  <Button size="sm" variant="outline" onClick={() => setEditing(true)}>
                    <Pencil aria-hidden="true" />
                    {t('header.edit')}
                  </Button>
                )}
                <ShareRecordButton recordType={recordType} recordId={record.id} />
                {actions}
                <Slot name="record.detail.actions" props={slotProps} />
              </>
            }
            subtitle={
              subtitle && hasValue(subtitle.value) ? (
                subtitle.field ? (
                  <FieldValueDisplay field={subtitle.field} value={subtitle.value} />
                ) : (
                  String(subtitle.value)
                )
              ) : undefined
            }
            owner={typeof owner.value === 'string' && owner.value ? <UserValue userId={owner.value} /> : undefined}
          />
          <Slot name="record.detail.header" props={slotProps} />
          </>
        }
        sidebar={
          <div className="grid min-w-0 gap-4">
            <RecordFieldsPanel
              fields={fields}
              layout={viewLayout}
              record={record}
              readOnly={readOnly}
              readOnlyFieldKeys={readOnlyFieldKeys}
              onSaveField={saveField}
            >
              <Slot name="record.fields.panel" props={slotProps} />
            </RecordFieldsPanel>
            <Slot name="record.detail.sidebar" props={slotProps} />
          </div>
        }
      >
        <RecordTabs tabs={resolvedTabs} defaultTab={defaultTab} />
      </RecordPageShell>
      {!readOnly && (
        <RecordFormDialog recordType={recordType} record={record} open={editing} onOpenChange={setEditing} />
      )}
    </DisplayRecordProvider>
  )
}
