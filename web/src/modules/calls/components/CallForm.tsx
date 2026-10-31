import { OptionGroup } from '@/components/ui/option-group'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { PhoneIncoming, PhoneOutgoing, Play, Square } from 'lucide-react'
import { useEffect, useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { cn } from '@/lib/utils'
import { formatCallDuration, splitDuration } from '../lib/format'
import type { CallOutcome } from '../types'
import type { CallFormValues } from './call-form-values'
import { Field } from '@/components/ui/field'

export interface CallFormProps {
  values: CallFormValues
  onChange: (values: CallFormValues) => void
  outcomes: CallOutcome[]
  outcomeError?: boolean
  disabled?: boolean
  autoStartTimer?: boolean
}

export function CallForm({ values, onChange, outcomes, outcomeError, disabled, autoStartTimer }: CallFormProps) {
  const { t } = useTranslation('calls')
  const formId = useId()
  const [timerStart, setTimerStart] = useState<number | null>(() => (autoStartTimer ? Date.now() : null))
  const [elapsed, setElapsed] = useState(0)

  useEffect(() => {
    if (timerStart === null) return
    const tick = () => setElapsed(Math.floor((Date.now() - timerStart) / 1000))
    tick()
    const handle = window.setInterval(tick, 1000)
    return () => window.clearInterval(handle)
  }, [timerStart])

  const toggleTimer = () => {
    if (timerStart === null) {
      setElapsed(0)
      setTimerStart(Date.now())
      return
    }
    const seconds = Math.floor((Date.now() - timerStart) / 1000)
    setTimerStart(null)
    onChange({ ...values, ...splitDuration(seconds) })
  }

  const set = (patch: Partial<CallFormValues>) => onChange({ ...values, ...patch })

  return (
    <div className="grid gap-4">
      <fieldset className="grid gap-1.5" disabled={disabled}>
        <legend className="mb-1.5 text-sm font-medium">{t('form.direction')}</legend>
        <SegmentedControl
          size="sm"
          className="self-start"
          value={values.direction}
          onValueChange={(direction) => set({ direction })}
          options={[
            { value: 'outbound', label: t('direction.outbound'), icon: <PhoneOutgoing aria-hidden="true" /> },
            { value: 'inbound', label: t('direction.inbound'), icon: <PhoneIncoming aria-hidden="true" /> },
          ]}
        />
      </fieldset>
      <fieldset className="grid gap-1.5" disabled={disabled}>
        <legend className="mb-1.5 text-sm font-medium">{t('form.outcome')}</legend>
        <OptionGroup
          size="sm"
          aria-invalid={outcomeError ? true : undefined}
          value={values.outcomeKey}
          onValueChange={(outcomeKey) => set({ outcomeKey })}
          options={outcomes.map((outcome) => ({ value: outcome.key, label: outcome.name }))}
        />
        {outcomeError && <p className="text-sm text-destructive">{t('form.outcomeRequired')}</p>}
      </fieldset>
      <div className="grid gap-2.5">
        <Label htmlFor={`${formId}-minutes`}>{t('form.duration')}</Label>
        <div className="flex flex-wrap items-center gap-2">
          <Input
            id={`${formId}-minutes`}
            className="w-20"
            inputMode="numeric"
            aria-label={t('form.minutes')}
            placeholder={t('form.minutesShort')}
            value={values.minutes}
            disabled={disabled || timerStart !== null}
            onChange={(event) => set({ minutes: event.target.value.replace(/\D/g, '') })}
          />
          <span aria-hidden="true">:</span>
          <Input
            className="w-20"
            inputMode="numeric"
            aria-label={t('form.seconds')}
            placeholder={t('form.secondsShort')}
            value={values.seconds}
            disabled={disabled || timerStart !== null}
            onChange={(event) => set({ seconds: event.target.value.replace(/\D/g, '').slice(0, 2) })}
          />
          <Button
            type="button"
            size="sm"
            variant="outline"
            disabled={disabled}
            onClick={toggleTimer}
            className={cn(timerStart !== null && 'tabular-nums')}
          >
            {timerStart !== null ? <Square aria-hidden="true" /> : <Play aria-hidden="true" />}
            {timerStart !== null
              ? t('form.stopTimer', { time: formatCallDuration(elapsed) })
              : t('form.startTimer')}
          </Button>
        </div>
      </div>
      <Field label={t('form.notes')}>
        {(fieldControl) => <Textarea
          {...fieldControl}
          rows={3}
          value={values.notes}
          disabled={disabled}
          placeholder={t('form.notesPlaceholder')}
          onChange={(event) => set({ notes: event.target.value })}
        />}
      </Field>
    </div>
  )
}
