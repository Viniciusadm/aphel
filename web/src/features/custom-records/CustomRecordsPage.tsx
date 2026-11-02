import { SegmentedControl } from '@/components/ui/segmented-control'
import { Kanban, List, Plus } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate, useSearchParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { useFieldSchema } from '@/features/records/api'

import { activeFields } from '@/features/records/fields'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { RecordFormDialog } from '@/features/records/form'
import { RecordListView, SearchBox } from '@/features/records/table'
import type { FieldSchema } from '@/features/records/types'
import { cn } from '@/lib/utils'
import { requiredRelations, useRelationships } from './api'
import { CustomObjectGate } from './CustomObjectGate'
import { CustomRecordCreateDialog } from './CustomRecordCreateDialog'
import { objectColor } from './icons'
import { ObjectIcon } from './ObjectIcon'
import { KanbanBoard } from './KanbanBoard'
import { CUSTOM_RECORD_READ_ONLY_KEYS, customRecordPath, objectPermission, type CustomObject } from './resources'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

const BOARD_PARAM = 'board'
const BOARD_FIELD_PARAM = 'board_field'
const BOARD_SEARCH_PARAM = 'board_search'
const KANBAN_MODE = 'kanban'

export function CustomRecordsPage() {
  return <CustomObjectGate>{(object) => <CustomRecordsContent key={object.key} object={object} />}</CustomObjectGate>
}

function createLayout(schema: FieldSchema | undefined): string[] | undefined {
  if (!schema) return undefined
  const editable = activeFields(schema).filter(
    (field) => !(field.is_system && CUSTOM_RECORD_READ_ONLY_KEYS.includes(field.key)),
  )
  const quick = editable.filter((field) => field.is_required || field.key === 'owner_id')
  return (quick.length > 0 ? quick : editable).map((field) => field.key)
}

function CustomRecordsContent({ object }: { object: CustomObject }) {
  const { t } = useTranslation('objects')
  const navigate = useNavigate()
  const { hasPermission } = useSession()
  const [searchParams, setSearchParams] = useSearchParams()
  const [creating, setCreating] = useState(false)
  const recordType = object.record_type
  const schema = useFieldSchema(recordType)
  const relationships = useRelationships(recordType)
  const required = useMemo(
    () => requiredRelations(relationships.data ?? [], recordType),
    [relationships.data, recordType],
  )
  const layout = useMemo(() => createLayout(schema.data), [schema.data])
  const selectFields = useMemo(
    () =>
      schema.data
        ? activeFields(schema.data).filter((field) => field.field_type === 'select' && field.options.length > 0)
        : [],
    [schema.data],
  )
  const canCreate = hasPermission(objectPermission(object, 'create'))
  const canEdit = hasPermission(objectPermission(object, 'update'))
  const kanban = searchParams.get(BOARD_PARAM) === KANBAN_MODE && selectFields.length > 0
  const boardField =
    selectFields.find((field) => field.key === searchParams.get(BOARD_FIELD_PARAM)) ?? selectFields[0] ?? null
  const boardSearch = searchParams.get(BOARD_SEARCH_PARAM) ?? ''

  const updateParams = (patch: Record<string, string | null>) =>
    setSearchParams(
      (current) => {
        const next = new URLSearchParams(current)
        for (const [key, value] of Object.entries(patch)) {
          if (value) next.set(key, value)
          else next.delete(key)
        }
        return next
      },
      { replace: true },
    )

  const newButton = canCreate ? (
    <Button size="sm" onClick={() => setCreating(true)}>
      <Plus aria-hidden="true" />
      {t('records.new', { label: object.label_singular })}
    </Button>
  ) : undefined
  const openRecord = (id: string) => navigate(customRecordPath(object.key, id))

  return (
    <Page className={cn(kanban && boardField && 'h-full')}>
<PageHeader
        icon={
          <span
            className="mt-0.5 flex size-9 shrink-0 items-center justify-center rounded-md border bg-card"
            style={{ color: objectColor(object.color) }}
          >
            <ObjectIcon icon={object.icon} className="size-5" aria-hidden />
          </span>
        }
        title={object.label_plural}
        description={<p>{object.description || t('records.description', { label: object.label_plural })}</p>}
        actions={
          selectFields.length > 0 && (
            <SegmentedControl
              aria-label={t('kanban.modeLabel')}
              size="sm"
              value={kanban ? 'board' : 'list'}
              onValueChange={(value) => updateParams({ [BOARD_PARAM]: value === 'board' ? KANBAN_MODE : null })}
              options={[
                { value: 'list', label: t('kanban.list'), icon: <List aria-hidden /> },
                { value: 'board', label: t('kanban.board'), icon: <Kanban aria-hidden /> },
              ]}
            />
          )
        }
      />
      {kanban && boardField ? (
        <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3">
          <div className="flex min-w-0 flex-wrap items-center gap-2">
            <SearchBox
              value={boardSearch}
              onChange={(value) => updateParams({ [BOARD_SEARCH_PARAM]: value })}
              className="w-full sm:w-64"
            />
            {selectFields.length > 1 && (
              <NativeSelect
                aria-label={t('kanban.field')}
                className="w-auto max-w-56"
                value={boardField.key}
                onChange={(event) => updateParams({ [BOARD_FIELD_PARAM]: event.target.value })}
              >
                {selectFields.map((field) => (
                  <option key={field.key} value={field.key}>
                    {field.label}
                  </option>
                ))}
              </NativeSelect>
            )}
            <div className="ml-auto flex flex-wrap items-center gap-2">{newButton}</div>
          </div>
          <KanbanBoard recordType={recordType} field={boardField} search={boardSearch} canEdit={canEdit} />
        </div>
      ) : (
        <RecordListView
          recordType={recordType}
          toolbarActions={newButton}
          emptyAction={newButton}
          onRowClick={(record) => openRecord(record.id)}
          archivedFilter
        />
      )}
      {canCreate &&
        (required.length > 0 ? (
          <CustomRecordCreateDialog
            recordType={recordType}
            open={creating}
            onOpenChange={setCreating}
            layout={layout}
            relations={required}
            title={t('records.dialogTitle', { label: object.label_singular })}
            description={t('records.dialogDescription')}
            onCreated={(record) => openRecord(record.id)}
          />
        ) : (
          <RecordFormDialog
            recordType={recordType}
            open={creating}
            onOpenChange={setCreating}
            layout={layout}
            title={t('records.dialogTitle', { label: object.label_singular })}
            description={t('records.dialogDescription')}
            onCreated={(record) => openRecord(record.id)}
          />
        ))}
    </Page>
  )
}
