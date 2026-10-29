import { useQueryClient } from '@tanstack/react-query'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback/notify'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { activitiesKeys } from '@/modules/activities/api'
import { callsKeys, useCallOutcomes } from './api'
import type { CallActivity, LogCallRequest } from './types'
import { cn } from '@/lib/utils'
import { submitWithOutbox } from '@/pwa/offline/outbox'
import { useOutboxFeedback } from '@/features/mobile/lib/outbox-feedback'
import { DictationField } from '@/features/mobile/components/DictationField'
import { RecordPicker, type PickedRecord } from '@/features/mobile/components/RecordPicker'
import { FormActions, FormBody, type QuickFormProps } from '@/features/mobile/components/QuickForm'

type CallDirection = 'outbound' | 'inbound'

const SECONDS_PER_MINUTE = 60
const MAX_CALL_MINUTES = 24 * 60

export interface CallQuickFormProps extends QuickFormProps {
  initialRecord: PickedRecord | null
}

export function CallQuickForm({ onDone, initialRecord }: CallQuickFormProps) {
  const { t } = useTranslation('calls', { keyPrefix: 'quickAdd' })
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
        path: '/v1/calls',
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
        <div className="grid grid-cols-2 gap-2">
          {(['outbound', 'inbound'] as const).map((option) => (
            <button
              key={option}
              type="button"
              aria-pressed={direction === option}
              onClick={() => setDirection(option)}
              className={cn(
                'min-h-11 rounded-md border px-3 text-sm',
                direction === option ? 'border-primary bg-primary/10 font-medium' : 'bg-background',
              )}
            >
              {t(`call.directions.${option}`)}
            </button>
          ))}
        </div>
      </fieldset>
      <fieldset className="grid gap-2">
        <legend className="mb-2 text-sm font-medium">{t('call.outcome')}</legend>
        <div className="flex flex-wrap gap-2">
          {activeOutcomes.map((outcome) => (
            <button
              key={outcome.key}
              type="button"
              aria-pressed={outcomeKey === outcome.key}
              onClick={() => setOutcomeKey(outcome.key)}
              className={cn(
                'min-h-11 rounded-full border px-4 text-sm',
                outcomeKey === outcome.key ? 'border-primary bg-primary/10 font-medium' : 'bg-background',
              )}
            >
              {outcome.name}
            </button>
          ))}
        </div>
        {touched && !outcomeKey && <p className="text-sm text-destructive">{t('call.outcomeRequired')}</p>}
      </fieldset>
      <div className="grid gap-2">
        <Label htmlFor={ids.minutes}>{t('call.minutes')}</Label>
        <Input
          id={ids.minutes}
          type="number"
          inputMode="decimal"
          min={0}
          max={MAX_CALL_MINUTES}
          step="any"
          value={minutes}
          onChange={(event) => setMinutes(event.target.value)}
          className="h-11 text-base"
          aria-invalid={minutesInvalid}
        />
        {minutesInvalid && <p className="text-sm text-destructive">{t('call.minutesInvalid')}</p>}
      </div>
      <DictationField id={ids.notes} label={t('call.notes')} value={notes} onChange={setNotes} maxLength={20_000} />
      <FormActions pending={pending} label={t('call.submit')} />
    </FormBody>
  )
}
