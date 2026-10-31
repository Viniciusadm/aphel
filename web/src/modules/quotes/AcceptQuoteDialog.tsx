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
import type { QuoteDetail } from './types'
import { Field } from '@/components/ui/field'

export interface AcceptQuoteDialogProps {
  quote: QuoteDetail
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function AcceptQuoteDialog({ quote, open, onOpenChange }: AcceptQuoteDialogProps) {
  const { t } = useTranslation('quotes')
  const ids = { name: useId(), email: useId(), note: useId() }
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
        <DialogHeader>
          <DialogTitle>{t('actions.acceptTitle')}</DialogTitle>
          <DialogDescription>{t('actions.acceptDescription')}</DialogDescription>
        </DialogHeader>
        <div className="grid gap-4">
          <div className="grid gap-2.5">
            <Label htmlFor={ids.name}>{t('actions.acceptName')}</Label>
            <Input placeholder={t('placeholders.name')} id={ids.name} value={name} maxLength={160} onChange={(event) => setName(event.target.value)} />
            <p className="text-xs text-muted-foreground">{t('actions.acceptNameHint')}</p>
          </div>
          <Field label={t('actions.acceptEmail')}>
            {(control) => <Input placeholder={t('placeholders.email')} {...control} type="email" value={email} onChange={(event) => setEmail(event.target.value)} />}
          </Field>
          <Field label={t('actions.acceptNote')}>
            {(control) => <Textarea placeholder={t('placeholders.note')} {...control} rows={3} maxLength={2000} value={note} onChange={(event) => setNote(event.target.value)} />}
          </Field>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t('actions.cancel')}
          </Button>
          <Button onClick={() => void accept()} disabled={mutations.accept.isPending}>
            {t('actions.accept')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
