import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { OptionGroup } from '@/components/ui/option-group'
import { useQueryClient } from '@tanstack/react-query'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback/notify'
import { NumberMaskInput } from '@/components/ui/number-mask-input'
import {activitiesKeys} from '@/sdk'
import { callsKeys, useCallOutcomes } from './api'
import type { CallActivity, LogCallRequest } from './types'
import { submitWithOutbox } from '@/pwa/offline/outbox'
import { DictationField, FormActions, FormBody, type PickedRecord, type QuickFormProps, RecordPicker, useOutboxFeedback } from '@/sdk'
import { Field } from '@/components/ui/field'

type CallDirection = 'outbound' | 'inbound'

const SECONDS_PER_MINUTE = 60
const MAX_CALL_MINUTES = 24 * 60

export interface CallQuickFormProps extends QuickFormProps {
  initialRecord: PickedRecord | null
}

export function CallQuickForm({ onDone, initialRecord }: CallQuickFormProps) {
  const { t } = useTranslation('telephony', { keyPrefix: 'quickAdd' })
  const queryClient = useQueryClient()
  const outcomes = useCallOutcomes()
  const activeOutcomes = (outcomes.data ?? []).filter((outcome) => outcome.is_active)
  const ids = { minutes: useId(), notes: useId() }
  const [record, setRecord] = useState<PickedRecord | null>(initialRecord)
  const [direction, setDirection] = useState<CallDirection>('outbound')
  const [outcomeKey, setOutcomeKey] = useState('')
  const [minutes, setMinutes] = useState('')
  const [notes, setNotes] = useState('')
  const [touched, setTouched] = useState(false)
  const minutesValue = minutes.trim() === '' ? null : Number(minutes)
  const minutesInvalid =
    minutesValue !== null && (!Number.isFinite(minutesValue) || minutesValue < 0 || minutesValue > MAX_CALL_MINUTES)

  const feedback = useOutboxFeedback()
  const [pending, setPending] = useState(false)

  const submit = async () => {
    setTouched(true)
    if (!outcomeKey || minutesInvalid) return
    setPending(true)
    try {
      const request: LogCallRequest = {
        outcome_key: outcomeKey,
        direction,
        duration_seconds: minutesValue === null ? null : Math.round(minutesValue * SECONDS_PER_MINUTE),
        notes: notes.trim() || null,
        record_type: record?.recordType ?? null,
        record_id: record?.recordId ?? null,
      }
      const result = await submitWithOutbox<CallActivity>({
        kind: 'call',
        path: '/v1/telephony',
        body: request,
        label: record ? t('call.outboxLabelFor', { title: record.title }) : t('call.outboxLabel'),
      })
      feedback(result, t('call.created'))
      void queryClient.invalidateQueries({ queryKey: activitiesKeys.all })
      void queryClient.invalidateQueries({ queryKey: callsKeys.all })
      onDone()
    } catch (error) {
      notifyError(error)
    } finally {
      setPending(false)
    }
  }

  return (
    <FormBody onSubmit={() => void submit()}>
      <RecordPicker value={record} onChange={setRecord} />
      <fieldset className="grid gap-2">
        <legend className="mb-2 text-sm font-medium">{t('call.direction')}</legend>
        <OptionGroup
          size="touch"
          value={direction}
          onValueChange={(next) => next && setDirection(next)}
          options={(['outbound', 'inbound'] as const).map((option) => ({ value: option, label: t(`call.directions.${option}`) }))}
        />
      </fieldset>
      <fieldset className="grid gap-2">
        <legend className="mb-2 text-sm font-medium">{t('call.outcome')}</legend>
        <OptionGroup
          size="touch"
          value={outcomeKey}
          onValueChange={setOutcomeKey}
          options={activeOutcomes.map((outcome) => ({ value: outcome.key, label: outcome.name }))}
        />
        {touched && !outcomeKey && <SharedFieldError>{t('call.outcomeRequired')}</SharedFieldError>}
      </fieldset>
      <Field label={t('call.minutes')}>
        {(control) => (
          <>
            <NumberMaskInput
              {...control}
              kind="quantity"
              maxLength={7}
              placeholder={t('placeholders.minutes')}
              value={minutes}
              onValueChange={setMinutes}
              className="h-11 text-base"
            />
            {minutesInvalid && <SharedFieldError>{t('call.minutesInvalid')}</SharedFieldError>}
          </>
        )}
      </Field>
      <DictationField id={ids.notes} label={t('call.notes')} value={notes} onChange={setNotes} maxLength={20_000} />
      <FormActions pending={pending} label={t('call.submit')} />
    </FormBody>
  )
}
