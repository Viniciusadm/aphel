import { useQueryClient } from '@tanstack/react-query'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { api } from '@/api'
import { notifySuccess, UnsavedChangesDialog } from '@/components/feedback'
import { RecordFieldInput } from '@/components/fields'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Skeleton } from '@/components/ui/skeleton'
import { recordsKeys, useFieldSchema } from '@/features/records/api'
import { fieldViolations, type RecordPayload } from '@/features/records/fields'
import { RecordForm, quickCreateFields } from '@/features/records/form'
import { savedLayout, useFormLayout } from '@/features/records/form/layout-api'
import { recordPath, requireRecordResource } from '@/features/records/resources'
import type { RecordItem } from '@/features/records/types'
import type { RequiredRelation } from './api'
import { Field } from '@/components/ui/field'

const RELATIONS_ERROR_PREFIX = 'relations.'

export interface CustomRecordCreateDialogProps {
  recordType: string
  open: boolean
  onOpenChange: (open: boolean) => void
  layout?: string[]
  relations: RequiredRelation[]
  title: string
  description: string
  onCreated?: (record: RecordItem) => void
}

interface BodyProps {
  recordType: string
  layout?: string[]
  relations: RequiredRelation[]
  onCreated?: (record: RecordItem) => void
  onDone: () => void
  onCancel: () => void
  onDirtyChange: (dirty: boolean) => void
}

function relationErrors(error: unknown): Record<string, string> | null {
  const details = typeof error === 'object' && error !== null ? (error as { details?: unknown }).details : null
  const violations = fieldViolations(details)
  if (violations.length === 0) return null
  if (!violations.every((violation) => violation.field.startsWith(RELATIONS_ERROR_PREFIX))) return null
  return Object.fromEntries(
    violations.map((violation) => [violation.field.slice(RELATIONS_ERROR_PREFIX.length), violation.message]),
  )
}

function CreateBody({ recordType, layout, relations, onCreated, onDone, onCancel, onDirtyChange }: BodyProps) {
  const { t } = useTranslation('objects')
  const formId = useId()
  const resource = requireRecordResource(recordType)
  const schema = useFieldSchema(recordType)
  const formLayout = useFormLayout(recordType, 'create')
  const queryClient = useQueryClient()
  const [links, setLinks] = useState<Record<string, string | null>>({})
  const [errors, setErrors] = useState<Record<string, string>>({})

  if (schema.isPending || (formLayout.isPending && !formLayout.isError)) {
    return (
      <div className="grid gap-4" aria-busy="true">
        <Skeleton className="h-9 w-full" />
        <Skeleton className="h-9 w-full" />
      </div>
    )
  }
  if (schema.isError) {
    return (
      <div className="grid gap-3">
        <p className="text-sm text-destructive">{t('records.formLoadError')}</p>
        <Button variant="outline" className="justify-self-start" onClick={() => void schema.refetch()}>
          {t('records.retry')}
        </Button>
      </div>
    )
  }

  const custom = savedLayout(formLayout.data)

  const submit = async (payload: RecordPayload) => {
    const missing = relations.filter((relation) => !links[relation.key])
    if (missing.length > 0) {
      setErrors(Object.fromEntries(missing.map((relation) => [relation.key, t('records.relationRequired')])))
      return
    }
    const body = {
      ...payload,
      relations: Object.fromEntries(relations.map((relation) => [relation.key, links[relation.key]])),
    }
    try {
      const record = await api.post<RecordItem>(recordPath(resource), body)
      void queryClient.invalidateQueries({ queryKey: recordsKeys.list(recordType) })
      void queryClient.invalidateQueries({ queryKey: recordsKeys.lookup(recordType) })
      notifySuccess(t('records.created'))
      onCreated?.(record)
      onDone()
    } catch (error) {
      const mapped = relationErrors(error)
      if (!mapped) throw error
      setErrors(mapped)
    }
  }

  return (
    <div className="grid gap-4">
      <fieldset className="grid gap-3 rounded-md border p-3">
        <legend className="px-1 text-sm font-medium">{t('records.relationsLegend')}</legend>
        {relations.map((relation) => {
          const inputId = `${formId}-${relation.key}`
          const error = errors[relation.key]
          return (
            <Field key={relation.key} id={inputId} label={<>{relation.label}
                <span className="text-destructive" aria-hidden>
                  {' *'}
                </span></>}>
              <RecordFieldInput
                id={inputId}
                targetRecordType={relation.targetType}
                value={links[relation.key] ?? null}
                invalid={Boolean(error)}
                aria-describedby={error ? `${inputId}-error` : undefined}
                onChange={(value) => {
                  setLinks((current) => ({ ...current, [relation.key]: value }))
                  setErrors((current) => {
                    const next = { ...current }
                    delete next[relation.key]
                    return next
                  })
                }}
              />
              {error && (
                <p id={`${inputId}-error`} className="text-sm text-destructive">
                  {error}
                </p>
              )}
            </Field>
          )
        })}
      </fieldset>
      <RecordForm
        schema={schema.data}
        fields={custom ? undefined : quickCreateFields(schema.data, layout)}
        formLayout={custom}
        mode="create"
        columns={1}
        guardUnsavedChanges={false}
        onDirtyChange={onDirtyChange}
        onCancel={onCancel}
        onSubmit={submit}
      />
    </div>
  )
}

export function CustomRecordCreateDialog({
  recordType,
  open,
  onOpenChange,
  layout,
  relations,
  title,
  description,
  onCreated,
}: CustomRecordCreateDialogProps) {
  const [dirty, setDirty] = useState(false)
  const [confirming, setConfirming] = useState(false)

  const close = () => {
    setDirty(false)
    setConfirming(false)
    onOpenChange(false)
  }
  const requestClose = () => (dirty ? setConfirming(true) : close())

  return (
    <>
      <Dialog open={open} onOpenChange={(next) => (next ? onOpenChange(true) : requestClose())}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{title}</DialogTitle>
            <DialogDescription>{description}</DialogDescription>
          </DialogHeader>
          {open && (
            <CreateBody
              recordType={recordType}
              layout={layout}
              relations={relations}
              onCreated={onCreated}
              onDone={close}
              onCancel={requestClose}
              onDirtyChange={setDirty}
            />
          )}
        </DialogContent>
      </Dialog>
      <UnsavedChangesDialog open={confirming} onStay={() => setConfirming(false)} onLeave={close} />
    </>
  )
}
