import { zodResolver } from '@hookform/resolvers/zod'
import { useQueryClient } from '@tanstack/react-query'
import { useId } from 'react'
import { Controller, useForm, useWatch } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { z } from 'zod'
import { isApiError } from '@/api'
import { useSession } from '@/app/session/session-context'
import { notifyError, notifySuccess, PageLoader, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { fieldViolations } from '@/features/auth/lib/api-errors'
import { FormRow } from '@/features/settings/organization/components/FormRow'
import { NativeSelect } from '@/features/settings/organization/components/NativeSelect'
import {
  FORECAST_METHODS,
  MAX_HORIZON_MONTHS,
  forecastApi,
  forecastKeys,
  useForecastSettings,
  type ForecastSettings,
} from '../api'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

const percentage = z.coerce.number<string | number>().int('settings.errors.percentage').min(1, 'settings.errors.percentage').max(100, 'settings.errors.percentage')

const schema = z
  .object({
    method: z.enum(['stage_probability', 'deal_probability', 'commit_only']),
    categoriesEnabled: z.boolean(),
    bestCase: percentage,
    commit: percentage,
    horizon: z.coerce.number<string | number>().int().min(1).max(MAX_HORIZON_MONTHS),
  })
  .refine((values) => values.commit > values.bestCase, {
    path: ['commit'],
    message: 'settings.errors.order',
  })

type FormInput = z.input<typeof schema>
type FormValues = z.output<typeof schema>

function toFormValues(settings: ForecastSettings): FormInput {
  return {
    method: settings.method,
    categoriesEnabled: settings.categories_enabled,
    bestCase: settings.best_case_min_probability,
    commit: settings.commit_min_probability,
    horizon: settings.horizon_months,
  }
}

const serverFields: Record<string, keyof FormInput> = {
  method: 'method',
  best_case_min_probability: 'bestCase',
  commit_min_probability: 'commit',
  horizon_months: 'horizon',
}

export function ForecastSettingsPage() {
  const { t } = useTranslation('forecast')
  const query = useForecastSettings()
  return (
    <Page>
      <PageHeader title={t('settings.title')} description={t('settings.description')} />
      {query.isPending ? (
        <PageLoader />
      ) : query.isError ? (
        <ErrorState onRetry={() => void query.refetch()} />
      ) : (
        <SettingsForm key={query.data.updated_at ?? 'default'} settings={query.data} />
      )}
    </Page>
  )
}

function SettingsForm({ settings }: { settings: ForecastSettings }) {
  const { t } = useTranslation('forecast')
  const formId = useId()
  const { hasPermission } = useSession()
  const canEdit = hasPermission('settings.manage')
  const queryClient = useQueryClient()
  const {
    register,
    handleSubmit,
    control,
    reset,
    setError,
    formState: { errors, isSubmitting, isDirty },
  } = useForm<FormInput, unknown, FormValues>({ resolver: zodResolver(schema), defaultValues: toFormValues(settings) })
  const method = useWatch({ control, name: 'method' })
  const categoriesEnabled = useWatch({ control, name: 'categoriesEnabled' })
  const bestCase = useWatch({ control, name: 'bestCase' })
  const commit = useWatch({ control, name: 'commit' })
  const showRanges = categoriesEnabled || method === 'commit_only'

  const onSubmit = handleSubmit(async (values) => {
    try {
      const saved = await forecastApi.updateSettings(
        {
          method: values.method,
          categories_enabled: values.categoriesEnabled,
          best_case_min_probability: values.bestCase,
          commit_min_probability: values.commit,
          horizon_months: values.horizon,
        },
        settings.updated_at,
      )
      queryClient.setQueryData(forecastKeys.settings(), saved)
      await queryClient.invalidateQueries({ queryKey: forecastKeys.all })
      reset(toFormValues(saved))
      notifySuccess(t('settings.saved'))
    } catch (error) {
      if (isApiError(error) && error.code === 'version_conflict') {
        notifyError(error, { title: t('settings.errors.conflict') })
        return
      }
      const violations = isApiError(error) ? fieldViolations(error.details) : []
      let handled = false
      for (const violation of violations) {
        const field = serverFields[violation.field]
        if (field) {
          setError(field, { message: 'settings.errors.invalid' })
          handled = true
        }
      }
      if (!handled) notifyError(error)
    }
  })

  const errorText = (message?: string) => (message ? t(message as 'settings.errors.invalid') : undefined)

  return (
    <form className="grid max-w-2xl gap-8" onSubmit={(event) => void onSubmit(event)} noValidate>
      <UnsavedChangesGuard when={canEdit && isDirty} />
      <fieldset className="grid gap-3" disabled={!canEdit || isSubmitting}>
        <legend className="mb-2 text-base font-semibold">{t('settings.method')}</legend>
        {FORECAST_METHODS.map((value) => (
          <label key={value} className="flex cursor-pointer items-start gap-3 rounded-md border p-3 has-[:checked]:border-primary">
            <input type="radio" value={value} className="mt-1 accent-[var(--primary)]" {...register('method')} />
            <span className="grid gap-0.5">
              <span className="text-sm font-medium">{t(`methods.${value}`)}</span>
              <span className="text-xs text-muted-foreground">{t(`methodHints.${value}`)}</span>
            </span>
          </label>
        ))}
      </fieldset>

      <fieldset className="grid gap-4" disabled={!canEdit || isSubmitting}>
        <legend className="mb-2 text-base font-semibold">{t('settings.categories')}</legend>
        <div className="flex items-start gap-3">
          <Controller
            control={control}
            name="categoriesEnabled"
            render={({ field }) => (
              <Checkbox
                id={`${formId}-categories`}
                checked={field.value}
                onCheckedChange={(checked) => field.onChange(checked === true)}
                disabled={!canEdit || isSubmitting}
                className="mt-0.5"
              />
            )}
          />
          <div className="grid gap-0.5">
            <Label htmlFor={`${formId}-categories`}>{t('settings.categoriesEnabled')}</Label>
            <p className="text-xs text-muted-foreground">{t('settings.categoriesHint')}</p>
          </div>
        </div>
        <div className="grid gap-4 sm:grid-cols-2">
          <FormRow id={`${formId}-best-case`} label={t('settings.bestCase')} error={errorText(errors.bestCase?.message)}>
            {(describedBy) => (
              <Input placeholder={t('placeholders.percent')}
                id={`${formId}-best-case`}
                type="number"
                min={1}
                max={100}
                inputMode="numeric"
                aria-describedby={describedBy}
                aria-invalid={errors.bestCase ? true : undefined}
                {...register('bestCase')}
              />
            )}
          </FormRow>
          <FormRow id={`${formId}-commit`} label={t('settings.commit')} error={errorText(errors.commit?.message)}>
            {(describedBy) => (
              <Input placeholder={t('placeholders.percent')}
                id={`${formId}-commit`}
                type="number"
                min={1}
                max={100}
                inputMode="numeric"
                aria-describedby={describedBy}
                aria-invalid={errors.commit ? true : undefined}
                {...register('commit')}
              />
            )}
          </FormRow>
        </div>
        {showRanges && (
          <ul className="grid gap-1 rounded-md border bg-muted/40 p-3 text-sm" aria-label={t('settings.rangesLabel')}>
            <li>{t('settings.ranges.pipeline', { max: Math.max(0, Number(bestCase) - 1) })}</li>
            <li>{t('settings.ranges.best_case', { min: Number(bestCase), max: Math.max(0, Number(commit) - 1) })}</li>
            <li>{t('settings.ranges.commit', { min: Number(commit) })}</li>
          </ul>
        )}
      </fieldset>

      <fieldset className="grid gap-4" disabled={!canEdit || isSubmitting}>
        <legend className="mb-2 text-base font-semibold">{t('settings.horizonTitle')}</legend>
        <FormRow id={`${formId}-horizon`} label={t('settings.horizon')} hint={t('settings.horizonHint')}>
          {(describedBy) => (
            <NativeSelect id={`${formId}-horizon`} className="sm:w-48" aria-describedby={describedBy} {...register('horizon')}>
              {Array.from({ length: MAX_HORIZON_MONTHS }, (_, index) => index + 1).map((value) => (
                <option key={value} value={value}>
                  {t('page.months', { count: value })}
                </option>
              ))}
            </NativeSelect>
          )}
        </FormRow>
      </fieldset>

      {canEdit && (
        <div className="flex justify-end">
          <Button type="submit" disabled={isSubmitting || !isDirty}>
            {t('settings.save')}
          </Button>
        </div>
      )}
    </form>
  )
}
