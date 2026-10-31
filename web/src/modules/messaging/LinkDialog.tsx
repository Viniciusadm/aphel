import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useQueryClient } from '@tanstack/react-query'
import { Loader2 } from 'lucide-react'
import { useState } from 'react'
import { notifyError, notifySuccess } from '@/components/feedback'
import { RecordFieldInput } from '@/components/fields'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { MESSAGING_KEY, messagingApi, messagingKeys, type Conversation } from './api'
import { errorText } from './errors'
import { LinkedRecordLabel } from './shared'
import { Field } from '@/components/ui/field'

interface LinkDialogProps {
  conversation: Conversation
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function LinkDialog({ conversation, open, onOpenChange }: LinkDialogProps) {
  const t = useModuleTranslation(MESSAGING_KEY)
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('links.title')}</DialogTitle>
          <DialogDescription>{t('links.description')}</DialogDescription>
        </DialogHeader>
        {open && <LinkForm conversation={conversation} onDone={() => onOpenChange(false)} />}
      </DialogContent>
    </Dialog>
  )
}

function LinkForm({ conversation, onDone }: { conversation: Conversation; onDone: () => void }) {
  const t = useModuleTranslation(MESSAGING_KEY)
  const queryClient = useQueryClient()
  const [contactId, setContactId] = useState<string | null>(conversation.contact?.id ?? null)
  const [dealId, setDealId] = useState<string | null>(conversation.deal?.id ?? null)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const contactLocked = Boolean(conversation.contact && !conversation.contact.accessible)
  const dealLocked = Boolean(conversation.deal && !conversation.deal.accessible)

  const save = async () => {
    setSaving(true)
    setError(null)
    try {
      await messagingApi.link(conversation.id, contactId, dealId)
      notifySuccess(t('links.saved'))
      await queryClient.invalidateQueries({ queryKey: messagingKeys.conversations() })
      onDone()
    } catch (caught) {
      const known = errorText(t, caught)
      setError(known)
      if (!known) notifyError(caught)
    } finally {
      setSaving(false)
    }
  }

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        void save()
      }}
    >
      <Field id={`link-contact-${conversation.id}`} label={t('links.contact')}>
        {contactLocked && conversation.contact ? (
          <LinkedRecordLabel recordType="contact" record={conversation.contact} />
        ) : (
          <RecordFieldInput
            id={`link-contact-${conversation.id}`}
            targetRecordType="contact"
            value={contactId}
            onChange={setContactId}
          />
        )}
      </Field>
      <Field id={`link-deal-${conversation.id}`} label={t('links.deal')}>
        {dealLocked && conversation.deal ? (
          <LinkedRecordLabel recordType="deal" record={conversation.deal} />
        ) : (
          <RecordFieldInput
            id={`link-deal-${conversation.id}`}
            targetRecordType="deal"
            value={dealId}
            onChange={setDealId}
          />
        )}
      </Field>
      {error && <SharedFieldError>{error}</SharedFieldError>}
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onDone}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" disabled={saving}>
          {saving && <Loader2 className="animate-spin" aria-hidden="true" />}
          {t('actions.save')}
        </Button>
      </DialogFooter>
    </form>
  )
}
