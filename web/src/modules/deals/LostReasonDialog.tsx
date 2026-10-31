import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { useLostReasons } from '@/modules/pipelines/api'

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
  const noteId = useId()
  const errorId = useId()
  const missing = touched && !reasonId

  const submit = () => {
    setTouched(true)
    if (!reasonId) return
    onConfirm({ lostReasonId: reasonId, note: note.trim() || undefined })
  }

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        submit()
      }}
    >
      <DialogHeader>
        <DialogTitle>{t('lost.title')}</DialogTitle>
        <DialogDescription>
          {dealTitle ? t('lost.descriptionFor', { title: dealTitle }) : t('lost.description')}
        </DialogDescription>
      </DialogHeader>
      <div className="grid gap-2">
        <Label htmlFor={selectId}>{t('lost.reason')}</Label>
        {reasons.isPending ? (
          <Skeleton className="h-9 w-full" />
        ) : (
          <select
            id={selectId}
            value={reasonId}
            onChange={(event) => setReasonId(event.target.value)}
            aria-invalid={missing || undefined}
            aria-describedby={missing ? errorId : undefined}
            className="h-9 w-full rounded-md border bg-background px-3 text-sm"
          >
            <option value="">{t('lost.choose')}</option>
            {(reasons.data ?? []).map((reason) => (
              <option key={reason.id} value={reason.id}>
                {reason.name}
              </option>
            ))}
          </select>
        )}
        {missing && (
          <p id={errorId} role="alert" className="text-sm text-destructive">
            {t('lost.required')}
          </p>
        )}
        {reasons.isError && <p className="text-sm text-destructive">{t('lost.loadError')}</p>}
      </div>
      <div className="grid gap-2">
        <Label htmlFor={noteId}>{t('lost.note')}</Label>
        <Textarea placeholder={t('placeholders.note')} id={noteId} value={note} maxLength={5000} onChange={(event) => setNote(event.target.value)} />
      </div>
      <DialogFooter>
        <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={pending}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" variant="destructive" disabled={pending}>
          {t('lost.confirm')}
        </Button>
      </DialogFooter>
    </form>
  )
}

export function LostReasonDialog({ open, onOpenChange, ...props }: LostReasonDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>{open && <LostReasonForm {...props} onOpenChange={onOpenChange} />}</DialogContent>
    </Dialog>
  )
}
