import { useQueryClient } from '@tanstack/react-query'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { notifyError } from '@/components/feedback/notify'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { CONTACT_RECORD_TYPE, contactResource } from './resource'
import { emitModuleEvent } from '@/kernel/modules/events'
import { recordsKeys } from '@/features/records/api'
import type { RecordItem } from '@/features/records/types'
import { submitWithOutbox } from '@/pwa/offline/outbox'
import { useOutboxFeedback } from '@/features/mobile/lib/outbox-feedback'
import { FormActions, FormBody, type QuickFormProps } from '@/features/mobile/components/QuickForm'

function splitName(name: string): { first_name: string; last_name: string | null } {
  const parts = name.trim().split(/\s+/)
  const [first = '', ...rest] = parts
  return { first_name: first, last_name: rest.length > 0 ? rest.join(' ') : null }
}

export function ContactQuickForm({ onDone }: QuickFormProps) {
  const { t } = useTranslation('contacts', { keyPrefix: 'quickAdd' })
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const ids = { name: useId(), phone: useId(), email: useId() }
  const [name, setName] = useState('')
  const [phone, setPhone] = useState('')
  const [email, setEmail] = useState('')
  const [touched, setTouched] = useState(false)
  const nameMissing = name.trim() === ''

  const feedback = useOutboxFeedback()
  const [pending, setPending] = useState(false)

  const submit = async () => {
    setTouched(true)
    if (nameMissing) return
    setPending(true)
    try {
      const result = await submitWithOutbox<RecordItem>({
        kind: 'contact',
        path: contactResource.basePath,
        body: { ...splitName(name), phone: phone.trim() || null, email: email.trim() || null },
        label: t('contact.outboxLabel', { name: name.trim() }),
      })
      feedback(result, t('contact.created', { name: name.trim() }))
      void queryClient.invalidateQueries({ queryKey: [...recordsKeys.all, CONTACT_RECORD_TYPE] })
      onDone()
      if (result.status !== 'sent') return
      emitModuleEvent('record.created', { recordType: CONTACT_RECORD_TYPE, record: result.data })
      const path = contactResource.detailPath?.(result.data.id)
      if (path) void navigate(path)
    } catch (error) {
      notifyError(error)
    } finally {
      setPending(false)
    }
  }

  return (
    <FormBody onSubmit={() => void submit()}>
      <div className="grid gap-2">
        <Label htmlFor={ids.name}>{t('contact.name')}</Label>
        <Input
          id={ids.name}
          value={name}
          onChange={(event) => setName(event.target.value)}
          autoComplete="name"
          autoCapitalize="words"
          className="h-11 text-base"
          aria-invalid={touched && nameMissing}
          required
        />
        {touched && nameMissing && <p className="text-sm text-destructive">{t('contact.nameRequired')}</p>}
      </div>
      <div className="grid gap-2">
        <Label htmlFor={ids.phone}>{t('contact.phone')}</Label>
        <Input
          id={ids.phone}
          type="tel"
          inputMode="tel"
          value={phone}
          onChange={(event) => setPhone(event.target.value)}
          autoComplete="tel"
          className="h-11 text-base"
        />
      </div>
      <div className="grid gap-2">
        <Label htmlFor={ids.email}>{t('contact.email')}</Label>
        <Input
          id={ids.email}
          type="email"
          inputMode="email"
          value={email}
          onChange={(event) => setEmail(event.target.value)}
          autoComplete="email"
          autoCapitalize="none"
          className="h-11 text-base"
        />
      </div>
      <FormActions pending={pending} label={t('contact.submit')} />
    </FormBody>
  )
}
