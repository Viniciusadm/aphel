import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useQueryClient } from '@tanstack/react-query'
import { useId, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { FieldInput } from '@/components/fields'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { type BulkActionContext, NativeSelect, recordsKeys, useFieldSchema, useOrganizationUsers } from '@/sdk'
import type { ReferenceOption } from '@/kernel/modules/define'
import { useReferenceQueryKeys } from '@/kernel/modules/reference-options'
import { ReferenceOptions } from '@/kernel/modules/ReferenceOptions'
import {
  MAX_EXPORT_IDS,
  bulkApi,
  bulkKeys,
  isFinished,
  selectionOf,
  type BulkAction,
  type BulkExportFormat,
  type BulkOperation,
  type BulkRecordType,
} from './api'
import { BulkResult } from './BulkResult'
import { bulkEditableFields } from './fields'
import { Field } from '@/components/ui/field'

export type BulkActionKind =
  | 'assign_owner'
  | 'set_field'
  | 'tags'
  | 'move_stage'
  | 'enroll_in_sequence'
  | 'archive'
  | 'unarchive'
  | 'delete'
  | 'export'

export interface BulkActionDialogProps {
  kind: BulkActionKind
  context: BulkActionContext
  recordType: BulkRecordType
  onClose: () => void
}

export function BulkActionDialog({ kind, context, recordType, onClose }: BulkActionDialogProps) {
  const { t } = useTranslation('bulk_actions')
  const queryClient = useQueryClient()
  const formId = useId()
  const [operation, setOperation] = useState<BulkOperation | null>(null)
  const [submitting, setSubmitting] = useState(false)
  const [draft, setDraft] = useState<BulkAction | null>(kind === 'archive' || kind === 'unarchive' || kind === 'delete' ? { action: kind } : null)
  const [format, setFormat] = useState<BulkExportFormat>('xlsx')
  const count = context.total
  const referenceKeys = useReferenceQueryKeys(kind === 'enroll_in_sequence' ? ['tag', 'sequence'] : ['tag'])

  const close = () => {
    if (operation) {
      void queryClient.invalidateQueries({ queryKey: recordsKeys.list(recordType) })
      void queryClient.invalidateQueries({ queryKey: bulkKeys.all })
      for (const queryKey of referenceKeys) void queryClient.invalidateQueries({ queryKey })
      context.clearSelection()
    }
    onClose()
  }

  const exportTooLarge = kind === 'export' && !context.allMatching && context.selectedIds.length > MAX_EXPORT_IDS

  const submit = async () => {
    setSubmitting(true)
    try {
      if (kind === 'export') {
        await bulkApi.export(recordType, format, selectionOf(context))
        notifySuccess(t('export.started'), { description: t('export.startedDescription') })
        onClose()
        return
      }
      if (!draft) return
      const result = await bulkApi.start(recordType, draft, selectionOf(context))
      setOperation(result)
    } catch (error) {
      notifyError(error)
    } finally {
      setSubmitting(false)
    }
  }

  const running = operation !== null && !isFinished(operation.status)
  const destructive = kind === 'delete'

  return (
    <Dialog open onOpenChange={(open) => !open && !submitting && close()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t(`titles.${kind}`)}</DialogTitle>
          <DialogDescription>
            {context.allMatching ? t('scope.allMatching', { count }) : t('scope.selected', { count })}
          </DialogDescription>
        </DialogHeader>
        {operation ? (
          <BulkResult operation={operation} recordType={recordType} />
        ) : (
          <form
            id={formId}
            className="grid gap-4"
            noValidate
            onSubmit={(event) => {
              event.preventDefault()
              void submit()
            }}
          >
            {kind === 'assign_owner' && <OwnerForm onChange={setDraft} />}
            {kind === 'tags' && <TagsForm recordType={recordType} onChange={setDraft} />}
            {kind === 'set_field' && <FieldForm recordType={recordType} onChange={setDraft} />}
            {kind === 'move_stage' && <StageForm onChange={setDraft} />}
            {kind === 'enroll_in_sequence' && <SequenceForm onChange={setDraft} />}
            {kind === 'export' && (
              <Field label={t('export.format')}>
                {(control) => <>
                  <NativeSelect
                  {...control}
                  value={format}
                  onChange={(event) => setFormat(event.target.value as BulkExportFormat)}
                >
                  <option value="xlsx">XLSX</option>
                  <option value="csv">CSV</option>
                </NativeSelect>
                {exportTooLarge && (
                  <SharedFieldError>
                    {t('export.tooMany', { max: MAX_EXPORT_IDS })}
                  </SharedFieldError>
                )}
                </>}
              </Field>
            )}
            {(kind === 'archive' || kind === 'unarchive' || kind === 'delete') && (
              <p className="text-sm text-muted-foreground">{t(`confirm.${kind}`, { count })}</p>
            )}
            <p className="text-xs text-muted-foreground">{t('scope.permissionNote')}</p>
          </form>
        )}
        <DialogFooter>
          {operation ? (
            <Button onClick={close}>{running ? t('buttons.closeRunning') : t('buttons.close')}</Button>
          ) : (
            <>
              <Button variant="outline" onClick={close} disabled={submitting}>
                {t('buttons.cancel')}
              </Button>
              <Button
                type="submit"
                form={formId}
                variant={destructive ? 'destructive' : 'default'}
                disabled={submitting || exportTooLarge || (kind !== 'export' && !draft)}
              >
                {submitting ? t('buttons.applying') : t(kind === 'export' ? 'buttons.export' : 'buttons.apply')}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

interface DraftFormProps {
  onChange: (action: BulkAction | null) => void
}

function OwnerForm({ onChange }: DraftFormProps) {
  const { t } = useTranslation('bulk_actions')
  const users = useOrganizationUsers()
  const [ownerId, setOwnerId] = useState('')
  return (
    <Field label={t('owner.label')}>
      {(control) => <NativeSelect
        {...control}
        value={ownerId}
        disabled={users.isPending}
        onChange={(event) => {
          setOwnerId(event.target.value)
          onChange(event.target.value ? { action: 'assign_owner', owner_id: event.target.value } : null)
        }}
      >
        <option value="">{t('owner.placeholder')}</option>
        {(users.data ?? []).map((user) => (
          <option key={user.id} value={user.id}>
            {user.display_name}
          </option>
        ))}
      </NativeSelect>}
    </Field>
  )
}

function TagsForm({ recordType, onChange }: DraftFormProps & { recordType: BulkRecordType }) {
  const { t } = useTranslation('bulk_actions')
  const id = useId()
  const [mode, setMode] = useState<'add_tag' | 'remove_tag'>('add_tag')
  const [tagId, setTagId] = useState('')
  const emit = (nextMode: 'add_tag' | 'remove_tag', nextTag: string) =>
    onChange(nextTag ? { action: nextMode, tag_id: nextTag } : null)
  return (
    <>
      <fieldset className="grid gap-2">
        <legend className="text-sm font-medium">{t('tags.mode')}</legend>
        <div className="flex flex-wrap gap-4">
          {(['add_tag', 'remove_tag'] as const).map((option) => (
            <label key={option} className="flex items-center gap-2 text-sm">
              <input
                type="radio"
                name={`${id}-mode`}
                value={option}
                checked={mode === option}
                onChange={() => {
                  setMode(option)
                  emit(option, tagId)
                }}
              />
              {t(`tags.${option}`)}
            </label>
          ))}
        </div>
      </fieldset>
      <ReferenceOptions kind="tag" context={{ recordType }}>
        {({ options, isLoading }) => (
          <Field label={t('tags.label')}>
            {(control) => <>
              <NativeSelect
              {...control}
              value={tagId}
              disabled={isLoading}
              onChange={(event) => {
                setTagId(event.target.value)
                emit(mode, event.target.value)
              }}
            >
              <option value="">{t('tags.placeholder')}</option>
              {options.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </NativeSelect>
            {!isLoading && options.length === 0 && <p className="text-xs text-muted-foreground">{t('tags.empty')}</p>}
            </>}
          </Field>
        )}
      </ReferenceOptions>
    </>
  )
}

function FieldForm({ recordType, onChange }: DraftFormProps & { recordType: BulkRecordType }) {
  const { t } = useTranslation('bulk_actions')
  const schema = useFieldSchema(recordType)
  const fields = useMemo(() => (schema.data ? bulkEditableFields(schema.data, recordType) : []), [schema.data, recordType])
  const [fieldKey, setFieldKey] = useState('')
  const [value, setValue] = useState<unknown>(null)
  const field = fields.find((candidate) => candidate.key === fieldKey)
  const emit = (key: string, next: unknown) =>
    onChange(key ? { action: 'set_field', field: key, value: next ?? null } : null)
  return (
    <>
      <Field label={t('field.label')}>
        {(control) => <NativeSelect
          {...control}
          value={fieldKey}
          disabled={schema.isPending}
          onChange={(event) => {
            setFieldKey(event.target.value)
            setValue(null)
            emit(event.target.value, null)
          }}
        >
          <option value="">{t('field.placeholder')}</option>
          {fields.map((candidate) => (
            <option key={candidate.key} value={candidate.key}>
              {candidate.label}
            </option>
          ))}
        </NativeSelect>}
      </Field>
      {field && (
        <Field label={t('field.value')}>
          {(fieldControl) => <>
            <FieldInput
            key={field.key}
            id={fieldControl.id}
            field={field}
            value={value}
            onChange={(next) => {
              setValue(next)
              emit(field.key, next)
            }}
          />
          <p className="text-xs text-muted-foreground">{t('field.emptyHint')}</p>
          </>}
        </Field>
      )}
    </>
  )
}

function StageForm({ onChange }: DraftFormProps) {
  const { t } = useTranslation('bulk_actions')
  const [stageId, setStageId] = useState('')
  return (
    <ReferenceOptions kind="pipeline_stage">
      {({ options, isLoading }) => (
        <Field label={t('stage.label')}>
          {(fieldControl) => <>
            <NativeSelect
            id={fieldControl.id}
            value={stageId}
            disabled={isLoading}
            onChange={(event) => {
              setStageId(event.target.value)
              onChange(event.target.value ? { action: 'move_stage', stage_id: event.target.value } : null)
            }}
          >
            <option value="">{t('stage.placeholder')}</option>
            {groupedOptions(options).map(([group, items]) => (
              <optgroup key={group} label={group}>
                {items.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </optgroup>
            ))}
          </NativeSelect>
          <p className="text-xs text-muted-foreground">{t('stage.rulesHint')}</p>
          </>}
        </Field>
      )}
    </ReferenceOptions>
  )
}

function SequenceForm({ onChange }: DraftFormProps) {
  const { t } = useTranslation('bulk_actions')
  const [sequenceId, setSequenceId] = useState('')
  return (
    <ReferenceOptions kind="sequence">
      {({ options, isLoading }) => (
        <Field label={t('sequence.label')}>
          {(control) => <>
            <NativeSelect
            {...control}
            value={sequenceId}
            disabled={isLoading}
            onChange={(event) => {
              setSequenceId(event.target.value)
              onChange(event.target.value ? { action: 'enroll_in_sequence', sequence_id: event.target.value } : null)
            }}
          >
            <option value="">{t('sequence.placeholder')}</option>
            {options.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </NativeSelect>
          {!isLoading && options.length === 0 && <p className="text-xs text-muted-foreground">{t('sequence.empty')}</p>}
          <p className="text-xs text-muted-foreground">{t('sequence.hint')}</p>
          </>}
        </Field>
      )}
    </ReferenceOptions>
  )
}

function groupedOptions(options: ReferenceOption[]): [string, ReferenceOption[]][] {
  const groups = new Map<string, ReferenceOption[]>()
  for (const option of options) {
    const group = option.group ?? ''
    groups.set(group, [...(groups.get(group) ?? []), option])
  }
  return [...groups.entries()]
}
