import { DialogForm } from '@/components/ui/dialog-form'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifySuccess } from '@/components/feedback'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { generateQuotePdf, sendQuoteForSignature } from './modules'
import { notifyQuoteError } from './errors'
import type { QuoteDetail } from './types'
import { Field } from '@/components/ui/field'

export interface SignatureDialogProps {
  quote: QuoteDetail
  proposalsEnabled: boolean
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function SignatureDialog({ quote, proposalsEnabled, open, onOpenChange }: SignatureDialogProps) {
  const { t } = useTranslation('quotes')
  const [name, setName] = useState(quote.acceptance?.name ?? '')
  const [email, setEmail] = useState(quote.acceptance?.email ?? '')
  const [message, setMessage] = useState('')
  const [pending, setPending] = useState(false)

  const submit = async () => {
    setPending(true)
    try {
      const document = await generateQuotePdf(quote.deal_id, quote.id, null)
      await sendQuoteForSignature(
        quote.deal_id,
        quote.id,
        document.file_id,
        [{ name: name.trim(), email: email.trim() }],
        message.trim() || null,
      )
      notifySuccess(t('signature.sent'))
      onOpenChange(false)
    } catch (error) {
      notifyQuoteError(error)
    } finally {
      setPending(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogForm
          title={t('signature.title')}
          description={t('signature.description')}
          cancelLabel={t('actions.cancel')}
          submitLabel={t('signature.submit')}
          onCancel={() => onOpenChange(false)}
          noValidate
          onSubmit={() => void submit()}
          pending={pending}
      submitDisabled={!proposalsEnabled || pending || !name.trim() || !email.trim()}
        >
        {proposalsEnabled ? (
          <div className="grid gap-4">
            <Field label={t('signature.signerName')}>
              {(control) => <Input placeholder={t('placeholders.name')} {...control} value={name} onChange={(event) => setName(event.target.value)} />}
            </Field>
            <Field label={t('signature.signerEmail')}>
              {(control) => <Input placeholder={t('placeholders.email')} {...control} type="email" value={email} onChange={(event) => setEmail(event.target.value)} />}
            </Field>
            <Field label={t('signature.message')}>
              {(control) => <Textarea placeholder={t('placeholders.message')} {...control} rows={3} value={message} onChange={(event) => setMessage(event.target.value)} />}
            </Field>
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">{t('signature.needsProposals')}</p>
        )}
        </DialogForm>
      </DialogContent>
    </Dialog>
  )
}
