import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { DateTimeFieldInput } from '@/components/fields/DateTimeFieldInput'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { DialogForm } from '@/components/ui/dialog-form'
import { Field } from '@/components/ui/field'
import { Textarea } from '@/components/ui/textarea'
import { useRecordFormatting } from '@/sdk'
import { useCreateReminder, useUpdateReminder } from './api'
import { presetInstant, REMINDER_PRESETS } from './presets'
import type { Reminder } from './types'

const NOTE_MAX = 500

export interface ReminderTarget {
  recordType: string
  recordId: string
  title?: string | null
}

interface ReminderDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  reminder?: Reminder
  target?: ReminderTarget
}

export function ReminderDialog({ open, onOpenChange, reminder, target }: ReminderDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        {open && <ReminderForm key={reminder?.id ?? 'new'} reminder={reminder} target={target} onClose={() => onOpenChange(false)} />}
      </DialogContent>
    </Dialog>
  )
}

function ReminderForm({ reminder, target, onClose }: { reminder?: Reminder; target?: ReminderTarget; onClose: () => void }) {
  const { t } = useTranslation('reminders')
  const formatting = useRecordFormatting()
  const create = useCreateReminder()
  const update = useUpdateReminder()
  const [when, setWhen] = useState<string | null>(reminder?.remind_at ?? null)
  const [note, setNote] = useState(reminder?.note ?? '')
  const [missing, setMissing] = useState(false)
  const pending = create.isPending || update.isPending
  const linked = reminder?.record ?? (target ? { record_type: target.recordType, record_id: target.recordId, title: target.title } : null)

  const submit = () => {
    if (!when) {
      setMissing(true)
      return
    }
    const trimmed = note.trim()
    const done = () => {
      notifySuccess(t(reminder ? 'toast.updated' : 'toast.created'))
      onClose()
    }
    if (reminder) {
      update.mutate({ id: reminder.id, body: { remind_at: when, note: trimmed === '' ? null : trimmed } }, { onSuccess: done, onError: notifyError })
      return
    }
    create.mutate(
      {
        remind_at: when,
        note: trimmed === '' ? undefined : trimmed,
        record_type: linked?.record_type,
        record_id: linked?.record_id,
      },
      { onSuccess: done, onError: notifyError },
    )
  }

  return (
    <DialogForm
      title={t(reminder ? 'dialog.editTitle' : 'dialog.newTitle')}
      description={linked ? t('dialog.linkedDescription', { title: linked.title ?? t('page.untitled') }) : t('dialog.description')}
      submitLabel={t(reminder ? 'dialog.save' : 'dialog.create')}
      cancelLabel={t('dialog.cancel')}
      pending={pending}
      onCancel={onClose}
      onSubmit={submit}
      noValidate
    >
      <Field label={t('dialog.when')} error={missing && !when ? t('dialog.whenRequired') : undefined}>
        {(control) => (
          <div className="grid gap-2">
            <DateTimeFieldInput
              id={control.id}
              value={when}
              onChange={(value) => {
                setWhen(value)
                setMissing(false)
              }}
              invalid={missing && !when}
              timeZone={formatting.timezone}
              locale={formatting.locale}
            />
            <div className="flex flex-wrap gap-2">
              {REMINDER_PRESETS.map((preset) => (
                <Button
                  key={preset}
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={() => {
                    setWhen(presetInstant(preset, formatting.timezone))
                    setMissing(false)
                  }}
                >
                  {t(`presets.${preset}`)}
                </Button>
              ))}
            </div>
          </div>
        )}
      </Field>
      <Field label={t('dialog.note')} description={t('dialog.noteHint', { max: NOTE_MAX })}>
        {(control) => <Textarea {...control} rows={3} maxLength={NOTE_MAX} value={note} onChange={(event) => setNote(event.target.value)} />}
      </Field>
    </DialogForm>
  )
}
