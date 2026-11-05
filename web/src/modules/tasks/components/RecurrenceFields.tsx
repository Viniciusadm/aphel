import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { NativeSelect } from '@/sdk'
import type {RecurrenceFrequency} from '@/sdk'
import {
  RECURRENCE_FREQUENCIES,
  RECURRENCE_WEEKDAYS,
  toggleWeekday,
  type RecurrenceEnd,
  type RecurrenceFormState,
} from '../lib/recurrence'
import { Field } from '@/components/ui/field'
import { ToggleField } from '@/components/ui/toggle-field'

export interface RecurrenceFieldsProps {
  value: RecurrenceFormState
  onChange: (value: RecurrenceFormState) => void
  error?: string | null
  disabled?: boolean
}

const RECURRENCE_ENDS: RecurrenceEnd[] = ['never', 'until', 'count']

export function RecurrenceFields({ value, onChange, error, disabled }: RecurrenceFieldsProps) {
  const { t } = useTranslation('tasks')
  const baseId = useId()
  const ids = {
    frequency: `${baseId}-frequency`,
    interval: `${baseId}-interval`,
    monthDay: `${baseId}-month-day`,
    end: `${baseId}-end`,
    until: `${baseId}-until`,
    count: `${baseId}-count`,
    error: `${baseId}-error`,
  }
  const set = (patch: Partial<RecurrenceFormState>) => onChange({ ...value, ...patch })
  const frequency = value.frequency === 'none' ? null : value.frequency
  const active = frequency !== null

  return (
    <fieldset className="grid min-w-0 gap-3" disabled={disabled} aria-describedby={error ? ids.error : undefined}>
      <div className="grid gap-4 sm:grid-cols-2">
        <Field label={t('recurrence.label')}>
          {(control) => <NativeSelect
            {...control}
            value={value.frequency}
            onChange={(event) => set({ frequency: event.target.value as RecurrenceFrequency | 'none' })}
          >
            <option value="none">{t('recurrence.none')}</option>
            {RECURRENCE_FREQUENCIES.map((frequency) => (
              <option key={frequency} value={frequency}>
                {t(`recurrence.frequency.${frequency}`)}
              </option>
            ))}
          </NativeSelect>}
        </Field>
        {frequency && (
          <Field label={t(`recurrence.interval.${frequency}`)}>
            {(control) => <Input placeholder={t('placeholders.interval')}
              {...control}
              inputMode="numeric"
              value={value.interval}
              onChange={(event) => set({ interval: event.target.value })}
            />}
          </Field>
        )}
      </div>
      {value.frequency === 'weekly' && (
        <div className="grid gap-1.5">
          <span className="text-sm font-medium">{t('recurrence.weekdaysLabel')}</span>
          <div className="flex flex-wrap gap-x-3 gap-y-2">
            {RECURRENCE_WEEKDAYS.map((weekday) => {
              return (
                <ToggleField key={weekday} label={t(`recurrence.weekdaysShort.${weekday}`)} checked={value.weekdays.includes(weekday)} onCheckedChange={() => set({ weekdays: toggleWeekday(value.weekdays, weekday) })} />
              )
            })}
          </div>
          <p className="text-xs text-muted-foreground">{t('recurrence.weekdaysHint')}</p>
        </div>
      )}
      {value.frequency === 'monthly' && (
        <Field label={t('recurrence.monthDay')}>
          {(control) => <>
            <Input placeholder={t('placeholders.monthDay')}
            {...control}
            inputMode="numeric"
            className="w-24"
            value={value.monthDay}
            onChange={(event) => set({ monthDay: event.target.value })}
          />
          <p className="text-xs text-muted-foreground">{t('recurrence.monthDayHint')}</p>
          </>}
        </Field>
      )}
      {active && (
        <div className="grid gap-4 sm:grid-cols-2">
          <Field label={t('recurrence.endLabel')}>
            {(control) => <NativeSelect
              {...control}
              value={value.end}
              onChange={(event) => set({ end: event.target.value as RecurrenceEnd })}
            >
              {RECURRENCE_ENDS.map((end) => (
                <option key={end} value={end}>
                  {t(`recurrence.end.${end}`)}
                </option>
              ))}
            </NativeSelect>}
          </Field>
          {value.end === 'until' && (
            <Field label={t('recurrence.untilLabel')}>
              {(control) => <Input
                {...control}
                type="date"
                value={value.until}
                onChange={(event) => set({ until: event.target.value })}
              />}
            </Field>
          )}
          {value.end === 'count' && (
            <Field label={t('recurrence.countLabel')}>
              {(control) => <Input placeholder={t('placeholders.count')}
                {...control}
                inputMode="numeric"
                value={value.count}
                onChange={(event) => set({ count: event.target.value })}
              />}
            </Field>
          )}
        </div>
      )}
      {error && (
        <SharedFieldError id={ids.error}>
          {error}
        </SharedFieldError>
      )}
    </fieldset>
  )
}
