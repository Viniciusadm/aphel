import { CheckCircle2, Hourglass, ShieldCheck } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/toast-store'
import { useSubmitApproval } from '../api'
import { approvalPath } from '../constants'
import { approvalErrorKey } from '../errors'
import type { ApprovalMoment, ApprovalSubmission } from '../types'
import { Field } from '@/components/ui/field'

export interface RequestApprovalDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  recordType: string
  recordId: string
  moment: ApprovalMoment
  targetId?: string | null
  onSubmitted?: (result: ApprovalSubmission) => void
}

export function RequestApprovalDialog({ open, onOpenChange, ...props }: RequestApprovalDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>{open && <RequestApprovalBody {...props} onOpenChange={onOpenChange} />}</DialogContent>
    </Dialog>
  )
}

function RequestApprovalBody({
  onOpenChange,
  recordType,
  recordId,
  moment,
  targetId,
  onSubmitted,
}: Omit<RequestApprovalDialogProps, 'open'>) {
  const { t } = useTranslation('approvals')
  const [comment, setComment] = useState('')
  const [result, setResult] = useState<ApprovalSubmission | null>(null)
  const submit = useSubmitApproval()

  const send = async () => {
    try {
      const submission = await submit.mutateAsync({
        record_type: recordType,
        record_id: recordId,
        moment,
        target_id: targetId ?? null,
        comment: comment.trim() || null,
      })
      setResult(submission)
      onSubmitted?.(submission)
    } catch (error) {
      toast({ title: t(approvalErrorKey(error)), variant: 'destructive' })
    }
  }

  if (result) {
    const Icon = result.outcome === 'pending' ? Hourglass : CheckCircle2
    return (
      <div className="grid gap-4">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Icon className="size-5 text-muted-foreground" aria-hidden="true" />
            {t(`submit.outcomes.${result.outcome}.title`)}
          </DialogTitle>
          <DialogDescription>{t(`submit.outcomes.${result.outcome}.description`)}</DialogDescription>
        </DialogHeader>
        {result.requests.length > 0 && (
          <ul className="grid gap-1 text-sm">
            {result.requests.map((request) => (
              <li key={request.id}>
                <Link to={approvalPath(request.id)} className="font-medium hover:underline">
                  {request.policy_name}
                </Link>
              </li>
            ))}
          </ul>
        )}
        <DialogFooter>
          <Button type="button" onClick={() => onOpenChange(false)}>
            {t('actions.close')}
          </Button>
        </DialogFooter>
      </div>
    )
  }

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        void send()
      }}
    >
      <DialogHeader>
        <DialogTitle className="flex items-center gap-2">
          <ShieldCheck className="size-5 text-muted-foreground" aria-hidden="true" />
          {t('submit.title')}
        </DialogTitle>
        <DialogDescription>{t(`submit.descriptions.${moment}`)}</DialogDescription>
      </DialogHeader>
      <Field label={t('submit.comment')}>
        {(control) => <Textarea
          {...control}
          value={comment}
          rows={4}
          maxLength={2000}
          placeholder={t('submit.commentPlaceholder')}
          onChange={(event) => setComment(event.target.value)}
        />}
      </Field>
      <DialogFooter>
        <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={submit.isPending}>
          {t('actions.close')}
        </Button>
        <Button type="submit" disabled={submit.isPending}>
          {t('submit.confirm')}
        </Button>
      </DialogFooter>
    </form>
  )
}
