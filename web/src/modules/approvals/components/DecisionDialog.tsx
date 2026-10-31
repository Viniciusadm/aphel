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
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/toast-store'
import { useApprovalDecision, type ApprovalDecisionAction } from '../api'
import { approvalErrorKey } from '../errors'
import type { ApprovalRequest } from '../types'

export interface DecisionDialogProps {
  request: ApprovalRequest
  action: ApprovalDecisionAction | null
  onOpenChange: (open: boolean) => void
  onDone?: (request: ApprovalRequest) => void
}

export function DecisionDialog({ request, action, onOpenChange, onDone }: DecisionDialogProps) {
  return (
    <Dialog open={action !== null} onOpenChange={onOpenChange}>
      <DialogContent>
        {action && (
          <DecisionForm key={action} request={request} action={action} onOpenChange={onOpenChange} onDone={onDone} />
        )}
      </DialogContent>
    </Dialog>
  )
}

interface DecisionFormProps extends Omit<DecisionDialogProps, 'action'> {
  action: ApprovalDecisionAction
}

function DecisionForm({ request, action, onOpenChange, onDone }: DecisionFormProps) {
  const { t } = useTranslation('approvals')
  const commentId = useId()
  const errorId = useId()
  const [comment, setComment] = useState('')
  const [missing, setMissing] = useState(false)
  const decision = useApprovalDecision()
  const required = action === 'reject'

  const submit = async () => {
    const trimmed = comment.trim()
    if (required && !trimmed) {
      setMissing(true)
      return
    }
    try {
      const updated = await decision.mutateAsync({
        requestId: request.id,
        action,
        comment: trimmed || null,
      })
      toast({ title: t(`decision.done.${action}`), variant: 'success' })
      onOpenChange(false)
      onDone?.(updated)
    } catch (error) {
      toast({ title: t(approvalErrorKey(error)), variant: 'destructive' })
    }
  }

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        void submit()
      }}
    >
      <DialogHeader>
        <DialogTitle>{t(`decision.${action}.title`)}</DialogTitle>
        <DialogDescription>{t(`decision.${action}.description`, { name: request.policy_name })}</DialogDescription>
      </DialogHeader>
      <div className="grid gap-2.5">
        <Label htmlFor={commentId}>{required ? t('decision.commentRequired') : t('decision.comment')}</Label>
        <Textarea placeholder={t('placeholders.comment')}
          id={commentId}
          value={comment}
          rows={4}
          maxLength={2000}
          aria-invalid={missing || undefined}
          aria-describedby={missing ? errorId : undefined}
          onChange={(event) => {
            setComment(event.target.value)
            setMissing(false)
          }}
        />
        {missing && (
          <p id={errorId} className="text-xs text-destructive">
            {t('decision.commentMissing')}
          </p>
        )}
      </div>
      <DialogFooter>
        <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={decision.isPending}>
          {t('actions.close')}
        </Button>
        <Button type="submit" variant={action === 'approve' ? 'default' : 'destructive'} disabled={decision.isPending}>
          {t(`decision.${action}.confirm`)}
        </Button>
      </DialogFooter>
    </form>
  )
}
