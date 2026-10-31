import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import type { RecurrenceFrequency } from '../api'
import {
  RECURRENCE_FREQUENCIES,
  RECURRENCE_WEEKDAYS,
  toggleWeekday,
  type RecurrenceEnd,
  type RecurrenceFormState,
} from '../lib/recurrence'
import { Field } from '@/components/ui/field'

export interface RecurrenceFieldsProps {
  value: RecurrenceFormState
  onChange: (value: RecurrenceFormState) => void
  error?: string | null
  disabled?: boolean
}

const RECURRENCE_ENDS: RecurrenceEnd[] = ['never', 'until', 'count']

export function RecurrenceFields({ value, onChange, error, disabled }: RecurrenceFieldsProps) {
  const { t } = useTranslation('activities')
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
        <div className="grid gap-2.5">
          <Label htmlFor={ids.frequency}>{t('recurrence.label')}</Label>
          <NativeSelect
            id={ids.frequency}
            value={value.frequency}
            onChange={(event) => set({ frequency: event.target.value as RecurrenceFrequency | 'none' })}
          >
            <option value="none">{t('recurrence.none')}</option>
            {RECURRENCE_FREQUENCIES.map((frequency) => (
              <option key={frequency} value={frequency}>
                {t(`recurrence.frequency.${frequency}`)}
              </option>
            ))}
          </NativeSelect>
        </div>
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
              const checkboxId = `${baseId}-weekday-${weekday}`
              return (
                <div key={weekday} className="flex items-center gap-1.5">
                  <Checkbox
                    id={checkboxId}
                    checked={value.weekdays.includes(weekday)}
                    onCheckedChange={() => set({ weekdays: toggleWeekday(value.weekdays, weekday) })}
                  />
                  <Label htmlFor={checkboxId} className="font-normal">
                    {t(`recurrence.weekdaysShort.${weekday}`)}
                  </Label>
                </div>
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
          <div className="grid gap-2.5">
            <Label htmlFor={ids.end}>{t('recurrence.endLabel')}</Label>
            <NativeSelect
              id={ids.end}
              value={value.end}
              onChange={(event) => set({ end: event.target.value as RecurrenceEnd })}
            >
              {RECURRENCE_ENDS.map((end) => (
                <option key={end} value={end}>
                  {t(`recurrence.end.${end}`)}
                </option>
              ))}
            </NativeSelect>
          </div>
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
        <p id={ids.error} className="text-sm text-destructive">
          {error}
        </p>
      )}
    </fieldset>
  )
}
