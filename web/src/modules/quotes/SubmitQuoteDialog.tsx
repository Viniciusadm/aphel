import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Textarea } from '@/components/ui/textarea'
import { useQuoteMutations } from './api'
import { notifyQuoteError } from './errors'
import type { QuoteDetail } from './types'
import { Field } from '@/components/ui/field'

export interface SubmitQuoteDialogProps {
  quote: QuoteDetail
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function SubmitQuoteDialog({ quote, open, onOpenChange }: SubmitQuoteDialogProps) {
  const { t } = useTranslation('quotes')
  const mutations = useQuoteMutations()
  const [comment, setComment] = useState('')

  const submit = async () => {
    try {
      const result = await mutations.submit.mutateAsync({ quoteId: quote.id, comment: comment.trim() || null })
      notifySuccess(result.status === 'approved' ? t('actions.approvedNow') : t('actions.submitted'))
      onOpenChange(false)
    } catch (error) {
      notifyQuoteError(error)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('actions.submitTitle')}</DialogTitle>
          <DialogDescription>{t('actions.submitDescription')}</DialogDescription>
        </DialogHeader>
        <Field label={t('actions.comment')}>
          {(control) => <Textarea placeholder={t('placeholders.comment')} {...control} rows={3} value={comment} onChange={(event) => setComment(event.target.value)} />}
        </Field>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t('actions.cancel')}
          </Button>
          <Button onClick={() => void submit()} disabled={mutations.submit.isPending}>
            {t('actions.submit')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
