import { DialogForm } from '@/components/ui/dialog-form'
import { useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { notifySuccess } from '@/components/feedback'
import {
  Dialog,
  DialogContent,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { NativeSelect } from '@/sdk'
import { DEFAULT_DUE_DAYS, REQUEST_KINDS, useRequestMutations, type RequestKind } from './api'
import { addDays, endOfDayIso, usePrivacyErrors } from './lib/shared'
import { Field } from '@/components/ui/field'

export interface RequestSelection {
  records: { record_type: string; id: string }[]
  items: { source: string; id: string }[]
}

interface CreateRequestDialogProps {
  term: string
  selection: RequestSelection
  onClose: () => void
  onCreated: () => void
}

export function CreateRequestDialog({ term, selection, onClose, onCreated }: CreateRequestDialogProps) {
  const { t } = useTranslation('privacy')
  const { create } = useRequestMutations()
  const notifyFailure = usePrivacyErrors()
  const [kind, setKind] = useState<RequestKind>('export')
  const [dueDate, setDueDate] = useState(addDays(DEFAULT_DUE_DAYS))
  const count = selection.records.length + selection.items.length

  const submit = (event: FormEvent) => {
    event.preventDefault()
    create.mutate(
      { kind, term, targets: selection, due_at: endOfDayIso(dueDate) },
      {
        onSuccess: () => {
          notifySuccess(t('create.created'))
          onCreated()
        },
        onError: notifyFailure,
      },
    )
  }

  return (
    <Dialog open onOpenChange={(open) => !open && !create.isPending && onClose()}>
      <DialogContent>
        <DialogForm
          onSubmit={submit}
          title={t('create.title')}
          description={<>{t('create.description', { count })}</>}
          cancelLabel={t('actions.cancel')}
          submitLabel={create.isPending ? t('actions.saving') : t('create.submit')}
          onCancel={onClose}
          pending={create.isPending}
          submitDisabled={create.isPending || count === 0}
        >
          <Field label={t('create.kind')} description={t(`kindDescriptions.${kind}`)}>
            {(control) => (
              <NativeSelect {...control} value={kind} onChange={(event) => setKind(event.target.value as RequestKind)}>
                {REQUEST_KINDS.map((value) => (
                  <option key={value} value={value}>
                    {t(`kinds.${value}`)}
                  </option>
                ))}
              </NativeSelect>
            )}
          </Field>
          <Field label={t('create.dueAt')} description={t('create.dueHint')}>
            {(control) => <Input {...control} type="date" value={dueDate} required onChange={(event) => setDueDate(event.target.value)} />}
          </Field>
          <p className="rounded-md border bg-muted/40 px-3 py-2 text-xs text-muted-foreground">{t('create.notice')}</p>
        </DialogForm>
      </DialogContent>
    </Dialog>
  )
}
