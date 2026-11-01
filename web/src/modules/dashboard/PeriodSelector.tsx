import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { PERIOD_PRESETS, isValidCustomRange } from './period'
import type { LayoutPeriod, PeriodPreset } from './types'
import { NativeSelect } from '@/components/ui/native-select'
import { Field } from '@/components/ui/field'

export interface PeriodSelectorProps {
  value: LayoutPeriod
  onChange: (period: LayoutPeriod) => void
}

function todayIso(): string {
  const now = new Date()
  const month = String(now.getMonth() + 1).padStart(2, '0')
  const day = String(now.getDate()).padStart(2, '0')
  return `${now.getFullYear()}-${month}-${day}`
}

export function PeriodSelector({ value, onChange }: PeriodSelectorProps) {
  const { t } = useTranslation('dashboard')
  const [preset, setPreset] = useState<PeriodPreset>(value.preset)
  const [from, setFrom] = useState(value.from ?? '')
  const [to, setTo] = useState(value.to ?? '')
  const [touched, setTouched] = useState(false)
  const [synced, setSynced] = useState(value)

  if (synced !== value) {
    setSynced(value)
    setPreset(value.preset)
    setFrom(value.from ?? '')
    setTo(value.to ?? '')
    setTouched(false)
  }

  const valid = isValidCustomRange(from, to)

  const choose = (next: PeriodPreset) => {
    setPreset(next)
    if (next === 'custom') {
      if (!from || !to) {
        const today = todayIso()
        setFrom(from || `${today.slice(0, 8)}01`)
        setTo(to || today)
      }
      return
    }
    onChange({ preset: next, from: null, to: null })
  }

  const apply = (event: FormEvent) => {
    event.preventDefault()
    setTouched(true)
    if (valid) onChange({ preset: 'custom', from, to })
  }

  return (
    <form className="flex flex-wrap items-end gap-2" onSubmit={apply}>
      <Field label={t('period.label')}>
        {(control) => <NativeSelect
          {...control}
          value={preset}
          onChange={(event) => choose(event.target.value as PeriodPreset)}
        >
          {PERIOD_PRESETS.map((option) => (
            <option key={option} value={option}>
              {t(`period.presets.${option}`)}
            </option>
          ))}
        </NativeSelect>}
      </Field>
      {preset === 'custom' && (
        <>
          <Field label={t('period.from')}>
            {(control) => <Input
              {...control}
              type="date"
              className="w-40"
              value={from}
              aria-invalid={touched && !valid}
              onChange={(event) => setFrom(event.target.value)}
            />}
          </Field>
          <Field label={t('period.to')}>
            {(control) => <Input
              {...control}
              type="date"
              className="w-40"
              value={to}
              aria-invalid={touched && !valid}
              onChange={(event) => setTo(event.target.value)}
            />}
          </Field>
          <Button type="submit" variant="outline">
            {t('period.apply')}
          </Button>
          {touched && !valid && (
            <SharedFieldError className="basis-full">
              {t('period.invalid')}
            </SharedFieldError>
          )}
        </>
      )}
    </form>
  )
}
