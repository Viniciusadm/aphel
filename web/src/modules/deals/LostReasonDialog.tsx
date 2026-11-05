import { useLostReasons } from '@/sdk'
import { DialogForm } from '@/components/ui/dialog-form'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Dialog,
  DialogContent,
} from '@/components/ui/dialog'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { NativeSelect } from '@/components/ui/native-select'
import { Field } from '@/components/ui/field'

export interface LostReasonChoice {
  lostReasonId: string
  note?: string
}

export interface LostReasonDialogProps {
  open: boolean
  dealTitle?: string
  pending?: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: (choice: LostReasonChoice) => void
}

function LostReasonForm({ dealTitle, pending, onOpenChange, onConfirm }: Omit<LostReasonDialogProps, 'open'>) {
  const { t } = useTranslation('deals')
  const reasons = useLostReasons()
  const [reasonId, setReasonId] = useState('')
  const [note, setNote] = useState('')
  const [touched, setTouched] = useState(false)
  const selectId = useId()
  const errorId = useId()
  const missing = touched && !reasonId

  const submit = () => {
    setTouched(true)
    if (!reasonId) return
    onConfirm({ lostReasonId: reasonId, note: note.trim() || undefined })
  }

  return (
    <DialogForm
      submitVariant="destructive"
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        submit()
      }}
      title={t('lost.title')}
      description={<>{dealTitle ? t('lost.descriptionFor', { title: dealTitle }) : t('lost.description')}</>}
      cancelLabel={t('actions.cancel')}
      submitLabel={t('lost.confirm')}
      onCancel={() => onOpenChange(false)}
      pending={pending}
      submitDisabled={pending}
    >
      <Field id={selectId} label={t('lost.reason')}>
        {reasons.isPending ? (
          <Skeleton className="h-9 w-full" />
        ) : (
          <NativeSelect
            id={selectId}
            value={reasonId}
            onChange={(event) => setReasonId(event.target.value)}
            aria-invalid={missing || undefined}
            aria-describedby={missing ? errorId : undefined}
            className="w-full"
          >
            <option value="">{t('lost.choose')}</option>
            {(reasons.data ?? []).map((reason) => (
              <option key={reason.id} value={reason.id}>
                {reason.name}
              </option>
            ))}
          </NativeSelect>
        )}
        {missing && (
          <SharedFieldError id={errorId}>
            {t('lost.required')}
          </SharedFieldError>
        )}
        {reasons.isError && <SharedFieldError>{t('lost.loadError')}</SharedFieldError>}
      </Field>
      <Field label={t('lost.note')}>
        {(fieldControl) => <Textarea placeholder={t('placeholders.note')} {...fieldControl} value={note} maxLength={5000} onChange={(event) => setNote(event.target.value)} />}
      </Field>
    </DialogForm>
  )
}

export function LostReasonDialog({ open, onOpenChange, ...props }: LostReasonDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>{open && <LostReasonForm {...props} onOpenChange={onOpenChange} />}</DialogContent>
    </Dialog>
  )
}
