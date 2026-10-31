import { zodResolver } from '@hookform/resolvers/zod'
import { useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useSaveAvailability } from '../api'
import { matchServerField, serverFieldViolations } from '../lib/errors'
import { FieldMessage } from '../lib/FieldMessage'
import { TimeZoneInput } from '../lib/TimeZoneInput'
import type { SchedulingAvailabilityResponse } from '../types'
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

interface AvailabilityFormProps {
  userId: string | null
  availability: SchedulingAvailabilityResponse
}

export function AvailabilityForm({ userId, availability }: AvailabilityFormProps) {
  const { t } = useTranslation('scheduling')
  const save = useSaveAvailability(userId)
  const {
    control,
    register,
    handleSubmit,
    setError,
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
    <div className="grid gap-2">
      <Label htmlFor={`availability-${name}`}>{label}</Label>
      <Input placeholder={t('placeholders.number')}
        id={`availability-${name}`}
        type="number"
        min={min}
        max={max}
        aria-invalid={errors[name] ? true : undefined}
        {...register(name, { valueAsNumber: true })}
      />
      <p className="text-xs text-muted-foreground">{hint}</p>
      <FieldMessage message={errors[name]?.message} />
    </div>
  )

  return (
    <form className="grid gap-6" onSubmit={onSubmit} noValidate>
      {!availability.configured && (
        <div className="flex items-center gap-2">
          <Badge variant="secondary">{t('availability.notConfigured')}</Badge>
          <span className="text-sm text-muted-foreground">{t('availability.notConfiguredHint')}</span>
        </div>
      )}
      <FieldMessage message={errors.root?.server?.message} />
      <div className="grid max-w-sm gap-2">
        <Label htmlFor="availability-timezone">{t('availability.timezone')}</Label>
        <TimeZoneInput
          id="availability-timezone"
          aria-invalid={errors.timezone ? true : undefined}
          {...register('timezone')}
        />
        <FieldMessage message={errors.timezone?.message} />
      </div>
      <WeeklyHoursEditor control={control} register={register} errors={errors} />
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {numberField(
          'minNoticeMinutes',
          t('availability.minNotice'),
          t('availability.minNoticeHint'),
          0,
          MAX_NOTICE_MINUTES,
        )}
        <div className="grid gap-2">
          <Label htmlFor="availability-maxPerDay">{t('availability.maxPerDay')}</Label>
          <Input
            id="availability-maxPerDay"
            type="number"
            min={1}
            max={MAX_PER_DAY}
            placeholder={t('availability.noLimit')}
            aria-invalid={errors.maxPerDay ? true : undefined}
            {...register('maxPerDay')}
          />
          <p className="text-xs text-muted-foreground">{t('availability.maxPerDayHint')}</p>
          <FieldMessage message={errors.maxPerDay?.message} />
        </div>
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
      </div>
      <div className="flex justify-end">
        <Button type="submit" disabled={isSubmitting || (availability.configured && !isDirty)}>
          {isSubmitting ? t('actions.saving') : t('actions.save')}
        </Button>
      </div>
    </form>
  )
}
