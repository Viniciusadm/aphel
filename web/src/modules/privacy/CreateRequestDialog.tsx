import { useId, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { DEFAULT_DUE_DAYS, REQUEST_KINDS, useRequestMutations, type RequestKind } from './api'
import { addDays, endOfDayIso, usePrivacyErrors } from './lib/shared'

export interface RequestSelection {
  records: { record_type: string; id: string }[]
  notes: string[]
  import_rows: { import_job_id: string; row_index: number }[]
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
  const dueId = useId()
  const dueHintId = useId()
  const [kind, setKind] = useState<RequestKind>('export')
  const [dueDate, setDueDate] = useState(addDays(DEFAULT_DUE_DAYS))
  const count = selection.records.length + selection.notes.length + selection.import_rows.length

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
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{t('create.title')}</DialogTitle>
          <DialogDescription>{t('create.description', { count })}</DialogDescription>
        </DialogHeader>
        <form className="grid gap-4" onSubmit={submit}>
          <fieldset className="grid gap-2">
            <legend className="mb-2 text-sm font-medium">{t('create.kind')}</legend>
            {REQUEST_KINDS.map((value) => (
              <label
                key={value}
                className="flex cursor-pointer items-start gap-3 rounded-md border px-3 py-2 has-[:checked]:border-primary"
              >
                <input
                  type="radio"
                  name="privacy-request-kind"
                  value={value}
                  className="mt-1 size-4 shrink-0 accent-primary"
                  checked={kind === value}
                  onChange={() => setKind(value)}
                />
                <span className="grid gap-0.5">
                  <span className="text-sm font-medium">{t(`kinds.${value}`)}</span>
                  <span className="text-xs text-muted-foreground">{t(`kindDescriptions.${value}`)}</span>
                </span>
              </label>
            ))}
          </fieldset>
          <div className="grid gap-2">
            <Label htmlFor={dueId}>{t('create.dueAt')}</Label>
            <Input
              id={dueId}
              type="date"
              value={dueDate}
              required
              aria-describedby={dueHintId}
              onChange={(event) => setDueDate(event.target.value)}
            />
            <p id={dueHintId} className="text-xs text-muted-foreground">
              {t('create.dueHint')}
            </p>
          </div>
          <p className="rounded-md border bg-muted/40 px-3 py-2 text-xs text-muted-foreground">{t('create.notice')}</p>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose} disabled={create.isPending}>
              {t('actions.cancel')}
            </Button>
            <Button type="submit" disabled={create.isPending || count === 0} aria-busy={create.isPending}>
              {create.isPending ? t('actions.saving') : t('create.submit')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
