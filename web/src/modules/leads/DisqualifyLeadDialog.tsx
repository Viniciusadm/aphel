import { useId, useState } from 'react'
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
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/toast-store'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useDisqualificationReasons, useLeadActions } from './api'
import { MAX_NOTE_LENGTH } from './constants'
import { lifecycleErrorMessage } from './errors'

export interface LeadTarget {
  id: string
  name: string
}

export interface DisqualifyLeadDialogProps {
  lead: LeadTarget | null
  onOpenChange: (open: boolean) => void
}

export function DisqualifyLeadDialog({ lead, onOpenChange }: DisqualifyLeadDialogProps) {
  const { t } = useTranslation('leads')
  return (
    <Dialog open={lead !== null} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('disqualify.title')}</DialogTitle>
          <DialogDescription>{t('disqualify.description', { name: lead?.name ?? '' })}</DialogDescription>
        </DialogHeader>
        {lead && <DisqualifyForm key={lead.id} lead={lead} onClose={() => onOpenChange(false)} />}
      </DialogContent>
    </Dialog>
  )
}

function DisqualifyForm({ lead, onClose }: { lead: LeadTarget; onClose: () => void }) {
  const { t } = useTranslation('leads')
  const formId = useId()
  const reasons = useDisqualificationReasons()
  const { disqualify } = useLeadActions()
  const [reasonId, setReasonId] = useState('')
  const [note, setNote] = useState('')
  const [reasonError, setReasonError] = useState(false)

  const submit = async () => {
    if (!reasonId) {
      setReasonError(true)
      return
    }
    try {
      await disqualify.mutateAsync({
        contactId: lead.id,
        body: { reason_id: reasonId, note: note.trim() || undefined },
      })
      notifySuccess(t('disqualify.done', { name: lead.name }))
      onClose()
    } catch (error) {
      toast({ title: lifecycleErrorMessage(error, t), variant: 'destructive' })
    }
  }

  return (
    <>
      <form
        id={formId}
        className="grid gap-4"
        noValidate
        onSubmit={(event) => {
          event.preventDefault()
          void submit()
        }}
      >
        <div className="grid gap-1.5">
          <Label htmlFor={`${formId}-reason`}>{t('disqualify.reason')}</Label>
          <NativeSelect
            id={`${formId}-reason`}
            value={reasonId}
            disabled={reasons.isPending}
            aria-invalid={reasonError ? true : undefined}
            onChange={(event) => {
              setReasonId(event.target.value)
              setReasonError(false)
            }}
          >
            <option value="">{t('disqualify.reasonPlaceholder')}</option>
            {(reasons.data ?? []).map((reason) => (
              <option key={reason.id} value={reason.id}>
                {reason.name}
              </option>
            ))}
          </NativeSelect>
          {reasonError && <p className="text-xs text-destructive">{t('disqualify.reasonRequired')}</p>}
          {reasons.isSuccess && reasons.data.length === 0 && (
            <p className="text-xs text-muted-foreground">{t('disqualify.noReasons')}</p>
          )}
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor={`${formId}-note`}>{t('disqualify.note')}</Label>
          <Textarea
            id={`${formId}-note`}
            value={note}
            maxLength={MAX_NOTE_LENGTH}
            rows={3}
            placeholder={t('disqualify.notePlaceholder')}
            onChange={(event) => setNote(event.target.value)}
          />
        </div>
      </form>
      <DialogFooter>
        <Button variant="outline" onClick={onClose} disabled={disqualify.isPending}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" form={formId} variant="destructive" disabled={disqualify.isPending}>
          {t('disqualify.submit')}
        </Button>
      </DialogFooter>
    </>
  )
}
