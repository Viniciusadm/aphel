import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
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
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { useFieldSchema } from '@/features/records/api'
import { FilterBuilder, emptyCondition, serializeFilter, toDraft, type DraftGroup } from '@/features/records/filters'
import type { FieldSchema } from '@/features/records/types'
import {
  MAX_SEGMENT_DESCRIPTION_LENGTH,
  MAX_SEGMENT_NAME_LENGTH,
  segmentsApi,
  type Segment,
  type SegmentRecordType,
} from './api'

export interface SegmentDialogProps {
  open: boolean
  recordType: SegmentRecordType
  editing: Segment | null
  onOpenChange: (open: boolean) => void
  onSaved: (segment: Segment) => void
}

export function SegmentDialog({ open, recordType, editing, onOpenChange, onSaved }: SegmentDialogProps) {
  const { t } = useTranslation('segments')
  const schema = useFieldSchema(recordType)
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t(editing ? 'dialog.editTitle' : 'dialog.newTitle')}</DialogTitle>
          <DialogDescription>{t('dialog.description')}</DialogDescription>
        </DialogHeader>
        {open &&
          (schema.isPending ? (
            <Skeleton className="h-40 w-full" />
          ) : schema.isError ? (
            <ErrorState onRetry={() => void schema.refetch()} />
          ) : (
            <SegmentForm
              key={editing?.id ?? 'new'}
              schema={schema.data}
              recordType={recordType}
              editing={editing}
              onClose={() => onOpenChange(false)}
              onSaved={onSaved}
            />
          ))}
      </DialogContent>
    </Dialog>
  )
}

interface SegmentFormProps {
  schema: FieldSchema
  recordType: SegmentRecordType
  editing: Segment | null
  onClose: () => void
  onSaved: (segment: Segment) => void
}

function initialDraft(editing: Segment | null): DraftGroup {
  const draft = toDraft(editing?.filter ?? null)
  return draft.children.length === 0 ? { ...draft, children: [emptyCondition()] } : draft
}

function SegmentForm({ schema, recordType, editing, onClose, onSaved }: SegmentFormProps) {
  const { t } = useTranslation('segments')
  const formId = useId()
  const [name, setName] = useState(editing?.name ?? '')
  const [description, setDescription] = useState(editing?.description ?? '')
  const [draft, setDraft] = useState<DraftGroup>(() => initialDraft(editing))
  const [errors, setErrors] = useState<{ name?: string; filter?: string }>({})
  const [saving, setSaving] = useState(false)

  const submit = async () => {
    const trimmed = name.trim()
    const filter = serializeFilter(draft, schema)
    const next: { name?: string; filter?: string } = {}
    if (!trimmed) next.name = t('dialog.nameRequired')
    if (!filter) next.filter = t('dialog.filterRequired')
    setErrors(next)
    if (next.name || next.filter || !filter) return
    setSaving(true)
    const body = { name: trimmed, description: description.trim() || null, filter }
    try {
      const saved = editing
        ? await segmentsApi.update(editing.id, body)
        : await segmentsApi.create({ ...body, record_type: recordType })
      notifySuccess(t(editing ? 'toast.updated' : 'toast.created', { name: trimmed }))
      onSaved(saved)
      onClose()
    } catch (error) {
      if (isApiError(error) && error.code === 'segment_name_taken') setErrors({ name: t('dialog.nameTaken') })
      else if (isApiError(error) && error.status === 422) setErrors({ filter: t('dialog.filterInvalid') })
      else notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  return (
    <>
      <form
        id={formId}
        className="grid min-w-0 gap-4"
        noValidate
        onSubmit={(event) => {
          event.preventDefault()
          void submit()
        }}
      >
        <div className="grid gap-1.5">
          <Label htmlFor={`${formId}-name`}>{t('dialog.name')}</Label>
          <Input
            id={`${formId}-name`}
            value={name}
            maxLength={MAX_SEGMENT_NAME_LENGTH}
            autoFocus
            aria-invalid={errors.name ? true : undefined}
            onChange={(event) => setName(event.target.value)}
          />
          {errors.name && <p className="text-xs text-destructive">{errors.name}</p>}
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor={`${formId}-description`}>{t('dialog.descriptionLabel')}</Label>
          <Textarea
            id={`${formId}-description`}
            value={description}
            maxLength={MAX_SEGMENT_DESCRIPTION_LENGTH}
            rows={2}
            onChange={(event) => setDescription(event.target.value)}
          />
        </div>
        <fieldset className="grid min-w-0 gap-2">
          <legend className="text-sm font-medium">{t('dialog.filter')}</legend>
          <p className="text-xs text-muted-foreground">{t('dialog.filterHint')}</p>
          <FilterBuilder schema={schema} value={draft} onChange={setDraft} />
          {errors.filter && (
            <p role="alert" className="text-xs text-destructive">
              {errors.filter}
            </p>
          )}
        </fieldset>
      </form>
      <DialogFooter>
        <Button variant="outline" onClick={onClose} disabled={saving}>
          {t('dialog.cancel')}
        </Button>
        <Button type="submit" form={formId} disabled={saving}>
          {t('dialog.save')}
        </Button>
      </DialogFooter>
    </>
  )
}
