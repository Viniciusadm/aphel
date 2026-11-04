import { Plus, Trash2 } from 'lucide-react'
import { useFieldArray, type Control, type FieldErrors, type UseFormRegister } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { FieldMessage } from '../lib/FieldMessage'
import { WEEKDAYS } from '../types'
import { MAX_INTERVALS, type AvailabilityFormValues } from './form'

interface WeeklyHoursEditorProps {
  control: Control<AvailabilityFormValues>
  register: UseFormRegister<AvailabilityFormValues>
  errors: FieldErrors<AvailabilityFormValues>
}

const DEFAULT_START = '09:00'
const DEFAULT_END = '18:00'

export function WeeklyHoursEditor({ control, register, errors }: WeeklyHoursEditorProps) {
  const { t } = useTranslation('booking')
  const intervals = useFieldArray({ control, name: 'intervals' })
  const full = intervals.fields.length >= MAX_INTERVALS

  return (
    <fieldset className="grid gap-3">
      <legend className="text-sm font-medium">{t('availability.weeklyHours')}</legend>
      <p className="text-sm text-muted-foreground">{t('availability.weeklyHoursHint')}</p>
      <FieldMessage message={errors.intervals?.message ?? errors.intervals?.root?.message} />
      <ul className="grid divide-y rounded-lg border">
        {WEEKDAYS.map((weekday) => {
          const entries = intervals.fields
            .map((field, index) => ({ field, index }))
            .filter(({ field }) => field.weekday === weekday)
          return (
            <li key={weekday} className="grid gap-2 p-3 sm:grid-cols-[8rem_1fr]">
              <span className="pt-2 text-sm font-medium">{t(`weekdays.${weekday}`)}</span>
              <div className="grid gap-2">
                {entries.length === 0 && (
                  <p className="pt-2 text-sm text-muted-foreground">{t('availability.unavailable')}</p>
                )}
                {entries.map(({ field, index }) => {
                  const intervalErrors = errors.intervals?.[index]
                  return (
                    <div key={field.id} className="grid gap-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <Input
                          className="w-24"
                          inputMode="numeric"
                          placeholder="HH:MM"
                          aria-label={t('availability.start')}
                          aria-invalid={intervalErrors?.start ? true : undefined}
                          {...register(`intervals.${index}.start`)}
                        />
                        <span className="text-sm text-muted-foreground">{t('availability.until')}</span>
                        <Input
                          className="w-24"
                          inputMode="numeric"
                          placeholder="HH:MM"
                          aria-label={t('availability.end')}
                          aria-invalid={intervalErrors?.end ? true : undefined}
                          {...register(`intervals.${index}.end`)}
                        />
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          aria-label={t('availability.removeInterval')}
                          onClick={() => intervals.remove(index)}
                        >
                          <Trash2 />
                        </Button>
                      </div>
                      <FieldMessage message={intervalErrors?.start?.message ?? intervalErrors?.end?.message} />
                    </div>
                  )
                })}
                <div>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    disabled={full}
                    onClick={() => intervals.append({ weekday, start: DEFAULT_START, end: DEFAULT_END })}
                  >
                    <Plus />
                    {t('availability.addInterval')}
                  </Button>
                </div>
              </div>
            </li>
          )
        })}
      </ul>
    </fieldset>
  )
}
