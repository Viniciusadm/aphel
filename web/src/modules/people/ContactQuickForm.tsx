import { CONTACT_RECORD_TYPE, contactResource } from './resource'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { notifyError } from '@/components/feedback/notify'
import { Input } from '@/components/ui/input'
import { PhoneFieldInput } from '@/components/fields/LazyPhoneFieldInput'
import { useRecordFormatting } from '@/sdk'
import { emitModuleEvent } from '@/kernel/modules/events'
import { FormActions, FormBody, type QuickFormProps, type RecordItem, recordsKeys, useOutboxFeedback } from '@/sdk'
import { submitWithOutbox } from '@/pwa/offline/outbox'
import { Field } from '@/components/ui/field'

function splitName(name: string): { first_name: string; last_name: string | null } {
  const parts = name.trim().split(/\s+/)
  const [first = '', ...rest] = parts
  return { first_name: first, last_name: rest.length > 0 ? rest.join(' ') : null }
}

export function ContactQuickForm({ onDone }: QuickFormProps) {
  const { t } = useTranslation('people', { keyPrefix: 'quickAdd' })
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [name, setName] = useState('')
  const [phone, setPhone] = useState('')
  const formatting = useRecordFormatting()
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
      <Field label={t('contact.name')}>
        {(control) => <>
          <Input placeholder={t('placeholders.name')}
          {...control}
          value={name}
          onChange={(event) => setName(event.target.value)}
          autoComplete="name"
          autoCapitalize="words"
          className="h-11 text-base"
          aria-invalid={touched && nameMissing}
          required
        />
        {touched && nameMissing && <SharedFieldError>{t('contact.nameRequired')}</SharedFieldError>}
        </>}
      </Field>
      <Field label={t('contact.phone')}>
        {(control) => <PhoneFieldInput
          {...control}
          locale={formatting.locale}
          fieldLabel={t('contact.phone')}
          value={phone || null}
          onChange={(next) => setPhone(next ?? '')}
          className="text-base"
        />}
      </Field>
      <Field label={t('contact.email')}>
        {(control) => <Input placeholder={t('placeholders.email')}
          {...control}
          type="email"
          inputMode="email"
          value={email}
          onChange={(event) => setEmail(event.target.value)}
          autoComplete="email"
          autoCapitalize="none"
          className="h-11 text-base"
        />}
      </Field>
      <FormActions pending={pending} label={t('contact.submit')} />
    </FormBody>
  )
}
