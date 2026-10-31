import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { generateQuotePdf, sendQuoteForSignature } from './modules'
import { notifyQuoteError } from './errors'
import type { QuoteDetail } from './types'

export interface SignatureDialogProps {
  quote: QuoteDetail
  proposalsEnabled: boolean
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function SignatureDialog({ quote, proposalsEnabled, open, onOpenChange }: SignatureDialogProps) {
  const { t } = useTranslation('quotes')
  const ids = { name: useId(), email: useId(), message: useId() }
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
        <DialogHeader>
          <DialogTitle>{t('signature.title')}</DialogTitle>
          <DialogDescription>{t('signature.description')}</DialogDescription>
        </DialogHeader>
        {proposalsEnabled ? (
          <div className="grid gap-4">
            <div className="grid gap-2.5">
              <Label htmlFor={ids.name}>{t('signature.signerName')}</Label>
              <Input placeholder={t('placeholders.name')} id={ids.name} value={name} onChange={(event) => setName(event.target.value)} />
            </div>
            <div className="grid gap-2.5">
              <Label htmlFor={ids.email}>{t('signature.signerEmail')}</Label>
              <Input placeholder={t('placeholders.email')} id={ids.email} type="email" value={email} onChange={(event) => setEmail(event.target.value)} />
            </div>
            <div className="grid gap-2.5">
              <Label htmlFor={ids.message}>{t('signature.message')}</Label>
              <Textarea placeholder={t('placeholders.message')} id={ids.message} rows={3} value={message} onChange={(event) => setMessage(event.target.value)} />
            </div>
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">{t('signature.needsProposals')}</p>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t('actions.cancel')}
          </Button>
          <Button
            onClick={() => void submit()}
            disabled={!proposalsEnabled || pending || !name.trim() || !email.trim()}
          >
            {t('signature.submit')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
