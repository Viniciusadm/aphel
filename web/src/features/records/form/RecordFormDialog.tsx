import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { api } from '@/api'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { cn } from '@/lib/utils'
import { recordsApi, recordsKeys, useFieldSchema, useIdempotencyKey } from '../api'
import type { RecordPayload, RecordValues } from '../fields'
import { getRecordResource, recordPath } from '../resources'
import type { RecordItem } from '../types'
import { RecordForm } from './RecordForm'
import { UnsavedChangesDialog } from '@/components/feedback'
import { quickCreateFields, type RecordFormLayout } from './lib/values'
import { savedLayout, useFormLayout } from './layout-api'
import { emitModuleEvent } from '@/kernel/modules/events'

export interface RecordFormDialogProps {
  recordType: string
  open: boolean
  onOpenChange: (open: boolean) => void
  layout?: RecordFormLayout
  defaultValues?: RecordValues
  presetValues?: RecordPayload
  record?: RecordItem
  title?: string
  description?: string
  onCreated?: (record: RecordItem) => void
  onUpdated?: (record: RecordItem) => void
}

interface DialogBodyProps {
  recordType: string
  layout?: RecordFormLayout
  defaultValues?: RecordValues
  presetValues?: RecordPayload
  record?: RecordItem
  onCreated?: (record: RecordItem) => void
  onUpdated?: (record: RecordItem) => void
  onDone: () => void
  onCancel: () => void
  onDirtyChange: (dirty: boolean) => void
}

function RecordFormDialogBody({
  recordType,
  layout,
  defaultValues,
  presetValues,
  record: editing,
  onCreated,
  onUpdated,
  onDone,
  onCancel,
  onDirtyChange,
}: DialogBodyProps) {
  const { t } = useTranslation('records-form')
  const resource = getRecordResource(recordType)
  const schema = useFieldSchema(recordType)
  const formLayout = useFormLayout(recordType, editing ? 'edit' : 'create', { enabled: Boolean(resource) })
  const queryClient = useQueryClient()
  const idempotencyKey = useIdempotencyKey()
  const create = useMutation({
    mutationFn: ({ payload, idempotencyKey }: { payload: RecordPayload; idempotencyKey: string }) =>
      recordsApi.createRecord(recordPath(resource!), payload, idempotencyKey),
  })
  const update = useMutation({
    mutationFn: (payload: RecordPayload) =>
      api.patch<RecordItem>(recordPath(resource!, editing!.id), payload, {
        headers: typeof editing?.updated_at === 'string' ? { 'If-Match': editing.updated_at } : undefined,
      }),
  })

  if (!resource) return <p className="text-sm text-destructive">{t('dialog.unavailable')}</p>
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
        <p className="text-sm text-destructive">{t('dialog.loadError')}</p>
        <Button variant="outline" className="justify-self-start" onClick={() => void schema.refetch()}>
          {t('actions.retry')}
        </Button>
      </div>
    )
  }

  const custom = savedLayout(formLayout.data)

  if (editing) {
    return (
      <RecordForm
        schema={schema.data}
        formLayout={custom}
        record={editing}
        mode="edit"
        columns={2}
        guardUnsavedChanges={false}
        onDirtyChange={onDirtyChange}
        onCancel={onCancel}
        onSubmit={async (payload) => {
          const updated = await update.mutateAsync(payload)
          queryClient.setQueryData(recordsKeys.detail(recordType, updated.id), updated)
          void queryClient.invalidateQueries({ queryKey: recordsKeys.list(recordType) })
          void queryClient.invalidateQueries({ queryKey: recordsKeys.lookup(recordType) })
          toast({ title: t('dialog.updated'), variant: 'success' })
          onUpdated?.(updated)
          onDone()
        }}
      />
    )
  }

  return (
    <RecordForm
      schema={schema.data}
      fields={custom ? undefined : quickCreateFields(schema.data, layout)}
      formLayout={custom}
      defaultValues={defaultValues}
      mode="create"
      columns={1}
      guardUnsavedChanges={false}
      onDirtyChange={onDirtyChange}
      onCancel={onCancel}
      onSubmit={async (payload) => {
        const body = { ...presetValues, ...payload }
        const record = await create.mutateAsync({ payload: body, idempotencyKey: idempotencyKey(body) })
        void queryClient.invalidateQueries({ queryKey: recordsKeys.list(recordType) })
        void queryClient.invalidateQueries({ queryKey: recordsKeys.lookup(recordType) })
        toast({ title: t('dialog.created'), variant: 'success' })
        emitModuleEvent('record.created', { recordType, record })
        onCreated?.(record)
        onDone()
      }}
    />
  )
}

export function RecordFormDialog({
  recordType,
  open,
  onOpenChange,
  layout,
  defaultValues,
  presetValues,
  record,
  title,
  description,
  onCreated,
  onUpdated,
}: RecordFormDialogProps) {
  const { t } = useTranslation('records-form')
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
        <DialogContent className={cn(record && 'sm:max-w-3xl')}>
          <DialogHeader>
            <DialogTitle>{title ?? t(record ? 'dialog.editTitle' : 'dialog.title')}</DialogTitle>
            <DialogDescription>{description ?? t(record ? 'dialog.editDescription' : 'dialog.description')}</DialogDescription>
          </DialogHeader>
          {open && (
            <RecordFormDialogBody
              recordType={recordType}
              layout={layout}
              defaultValues={defaultValues}
              presetValues={presetValues}
              record={record}
              onCreated={onCreated}
              onUpdated={onUpdated}
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
