import { DialogForm } from '@/components/ui/dialog-form'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifySuccess } from '@/components/feedback'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { useQuoteMutations } from './api'
import { notifyQuoteError } from './errors'
import type { QuoteDetail } from './types'
import { Field } from '@/components/ui/field'

export interface AcceptQuoteDialogProps {
  quote: QuoteDetail
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function AcceptQuoteDialog({ quote, open, onOpenChange }: AcceptQuoteDialogProps) {
  const { t } = useTranslation('quotes')
  const mutations = useQuoteMutations()
  const [name, setName] = useState('')
  const [email, setEmail] = useState('')
  const [note, setNote] = useState('')

  const accept = async () => {
    try {
      await mutations.accept.mutateAsync({
        quoteId: quote.id,
        request: {
          name: name.trim() || undefined,
          email: email.trim() || undefined,
          note: note.trim() || undefined,
        },
      })
      notifySuccess(t('actions.accepted'))
      onOpenChange(false)
    } catch (error) {
      notifyQuoteError(error)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogForm
          title={t('actions.acceptTitle')}
          description={t('actions.acceptDescription')}
          cancelLabel={t('actions.cancel')}
          submitLabel={t('actions.accept')}
          onCancel={() => onOpenChange(false)}
          noValidate
          onSubmit={() => void accept()}
          pending={mutations.accept.isPending}
          submitDisabled={mutations.accept.isPending}
        >
        <div className="grid gap-4">
          <Field label={t('actions.acceptName')}>
            {(control) => <>
              <Input placeholder={t('placeholders.name')} {...control} value={name} maxLength={160} onChange={(event) => setName(event.target.value)} />
            <p className="text-xs text-muted-foreground">{t('actions.acceptNameHint')}</p>
            </>}
          </Field>
          <Field label={t('actions.acceptEmail')}>
            {(control) => <Input placeholder={t('placeholders.email')} {...control} type="email" value={email} onChange={(event) => setEmail(event.target.value)} />}
          </Field>
          <Field label={t('actions.acceptNote')}>
            {(control) => <Textarea placeholder={t('placeholders.note')} {...control} rows={3} maxLength={2000} value={note} onChange={(event) => setNote(event.target.value)} />}
          </Field>
        </div>
        </DialogForm>
      </DialogContent>
    </Dialog>
  )
}
