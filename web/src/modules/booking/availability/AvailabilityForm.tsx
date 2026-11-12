import { zodResolver } from '@hookform/resolvers/zod'
import { useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { useSaveAvailability } from '../api'
import { matchServerField, serverFieldViolations } from '../lib/errors'
import { FieldMessage } from '../lib/FieldMessage'
import { TimeZoneInput } from '../lib/TimeZoneInput'
import type { BookingAvailabilityResponse } from '../types'
import {
  MAX_BUFFER_MINUTES,
  MAX_HORIZON_DAYS,
  MAX_NOTICE_MINUTES,
  MAX_PER_DAY,
  availabilityFormSchema,
  defaultValues,
  serverFieldPaths,
  toRequest,
  type AvailabilityFormValues,
} from './form'
import { WeeklyHoursEditor } from './WeeklyHoursEditor'
import { Field } from '@/components/ui/field'
import { SettingsCard, WIDE_FIELD } from '@/features/settings/components/SettingsCard'

interface AvailabilityFormProps {
  userId: string | null
  availability: BookingAvailabilityResponse
}

export function AvailabilityForm({ userId, availability }: AvailabilityFormProps) {
  const { t } = useTranslation('booking')
  const save = useSaveAvailability(userId)
  const {
    control,
    register,
    handleSubmit,
    setError,
    reset,
    formState: { errors, isSubmitting, isDirty },
  } = useForm<AvailabilityFormValues>({
    resolver: zodResolver(availabilityFormSchema),
    defaultValues: defaultValues(availability),
  })

  const onSubmit = handleSubmit(async (values) => {
    try {
      await save.mutateAsync({ request: toRequest(values), version: availability.updated_at ?? null })
      notifySuccess(t('availability.saved'))
    } catch (error) {
      let message = 'errors.generic'
      if (isApiError(error)) {
        if (error.code === 'version_conflict') message = 'errors.versionConflict'
        if (error.status === 403) message = 'availability.forbidden'
        for (const violation of serverFieldViolations(error)) {
          const path = matchServerField(violation.field, serverFieldPaths)
          if (path) setError(path, { type: 'server', message: 'errors.invalid' })
          message = 'errors.invalid'
        }
      }
      setError('root.server', { type: 'server', message })
    }
  })

  const numberField = (
    name: 'minNoticeMinutes' | 'bufferBeforeMinutes' | 'bufferAfterMinutes' | 'horizonDays',
    label: string,
    hint: string,
    min: number,
    max: number,
  ) => (
    <Field label={label}>
      {(control) => <>
        <Input placeholder={t('placeholders.number')}
        {...control}
        type="number"
        min={min}
        max={max}
        aria-invalid={errors[name] ? true : undefined}
        {...register(name, { valueAsNumber: true })}
      />
      <p className="text-xs text-muted-foreground">{hint}</p>
      <FieldMessage message={errors[name]?.message} />
      </>}
    </Field>
  )

  return (
    <form className="grid gap-6" onSubmit={onSubmit} noValidate>
      {!availability.configured && (
        <div className="flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
          <Badge variant="secondary">{t('availability.notConfigured')}</Badge>
          <span>{t('availability.notConfiguredHint')}</span>
        </div>
      )}
      <FieldMessage message={errors.root?.server?.message} />
      <SettingsCard title={t('availability.sections.hours')} description={t('availability.sections.hoursHint')}>
        <div className={WIDE_FIELD}>
          <Field label={t('availability.timezone')} className="max-w-sm">
            {(fieldControl) => <>
              <TimeZoneInput
              id={fieldControl.id}
              aria-invalid={errors.timezone ? true : undefined}
              {...register('timezone')}
            />
            <FieldMessage message={errors.timezone?.message} />
            </>}
          </Field>
        </div>
        <div className={WIDE_FIELD}>
          <WeeklyHoursEditor control={control} errors={errors} />
        </div>
      </SettingsCard>
      <SettingsCard title={t('availability.sections.rules')} description={t('availability.sections.rulesHint')}>
        {numberField(
          'minNoticeMinutes',
          t('availability.minNotice'),
          t('availability.minNoticeHint'),
          0,
          MAX_NOTICE_MINUTES,
        )}
        <Field label={t('availability.maxPerDay')}>
          {(fieldControl) => <>
            <Input
            id={fieldControl.id}
            type="number"
            min={1}
            max={MAX_PER_DAY}
            placeholder={t('availability.noLimit')}
            aria-invalid={errors.maxPerDay ? true : undefined}
            {...register('maxPerDay')}
          />
          <p className="text-xs text-muted-foreground">{t('availability.maxPerDayHint')}</p>
          <FieldMessage message={errors.maxPerDay?.message} />
          </>}
        </Field>
        {numberField('horizonDays', t('availability.horizon'), t('availability.horizonHint'), 1, MAX_HORIZON_DAYS)}
        {numberField(
          'bufferBeforeMinutes',
          t('availability.bufferBefore'),
          t('availability.bufferHint'),
          0,
          MAX_BUFFER_MINUTES,
        )}
        {numberField(
          'bufferAfterMinutes',
          t('availability.bufferAfter'),
          t('availability.bufferHint'),
          0,
          MAX_BUFFER_MINUTES,
        )}
      </SettingsCard>
      {(isDirty || !availability.configured) && (
        <div
          role="status"
          className="sticky bottom-4 z-10 mx-auto flex w-fit max-w-full flex-wrap items-center justify-center gap-3 rounded-lg border bg-card px-4 py-2 text-sm shadow-lg"
        >
          <span>{isDirty ? t('availability.unsaved') : t('availability.notConfiguredBar')}</span>
          <div className="flex items-center gap-2">
            {isDirty && (
              <Button type="button" variant="ghost" size="sm" disabled={isSubmitting} onClick={() => reset(defaultValues(availability))}>
                {t('actions.discard')}
              </Button>
            )}
            <Button type="submit" size="sm" disabled={isSubmitting}>
              {isSubmitting ? t('actions.saving') : t('actions.save')}
            </Button>
          </div>
        </div>
      )}
    </form>
  )
}
