import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { useQuoteMutations } from './api'
import { notifyQuoteError } from './errors'
import type { EmailAddressInput, QuoteDetail } from './types'

const LINK_PLACEHOLDER = '{{quote_link}}'

function addresses(text: string): EmailAddressInput[] {
  return text
    .split(/[,;\n]/)
    .map((part) => part.trim())
    .filter(Boolean)
    .map((address) => ({ address }))
}

function escapeHtml(text: string): string {
  return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

function toHtml(text: string): string {
  return text
    .split(/\n{2,}/)
    .map((paragraph) => `<p>${escapeHtml(paragraph).replace(/\n/g, '<br>')}</p>`)
    .join('')
}

export interface SendQuoteDialogProps {
  quote: QuoteDetail
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function SendQuoteDialog({ quote, open, onOpenChange }: SendQuoteDialogProps) {
  const { t } = useTranslation('quotes')
  const ids = { to: useId(), cc: useId(), subject: useId(), body: useId() }
  const mutations = useQuoteMutations()
  const [to, setTo] = useState('')
  const [cc, setCc] = useState('')
  const [subject, setSubject] = useState(() => t('send.defaultSubject', { number: quote.number, title: quote.title }))
  const [body, setBody] = useState(() =>
    t('send.defaultBody', { link: LINK_PLACEHOLDER, interpolation: { escapeValue: false } }),
  )

  const submit = async () => {
    try {
      await mutations.send.mutateAsync({
        quoteId: quote.id,
        request: { to: addresses(to), cc: addresses(cc), subject, body_html: toHtml(body) },
      })
      notifySuccess(t('send.sent'))
      onOpenChange(false)
    } catch (error) {
      notifyQuoteError(error)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>{t('send.title')}</DialogTitle>
          <DialogDescription>{t('send.description')}</DialogDescription>
        </DialogHeader>
        <div className="grid gap-4">
          <div className="grid gap-2.5">
            <Label htmlFor={ids.to}>{t('send.to')}</Label>
            <Input placeholder={t('placeholders.to')} id={ids.to} value={to} onChange={(event) => setTo(event.target.value)} />
            <p className="text-xs text-muted-foreground">{t('send.toHint')}</p>
          </div>
          <div className="grid gap-2.5">
            <Label htmlFor={ids.cc}>{t('send.cc')}</Label>
            <Input placeholder={t('placeholders.cc')} id={ids.cc} value={cc} onChange={(event) => setCc(event.target.value)} />
          </div>
          <div className="grid gap-2.5">
            <Label htmlFor={ids.subject}>{t('send.subject')}</Label>
            <Input placeholder={t('placeholders.subject')} id={ids.subject} value={subject} onChange={(event) => setSubject(event.target.value)} />
          </div>
          <div className="grid gap-2.5">
            <Label htmlFor={ids.body}>{t('send.body')}</Label>
            <Textarea placeholder={t('placeholders.body')} id={ids.body} rows={8} value={body} onChange={(event) => setBody(event.target.value)} />
            <p className="text-xs text-muted-foreground">
              {t('send.linkHint', { placeholder: LINK_PLACEHOLDER, interpolation: { escapeValue: false } })}
            </p>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t('actions.cancel')}
          </Button>
          <Button onClick={() => void submit()} disabled={mutations.send.isPending}>
            {t('send.submit')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
