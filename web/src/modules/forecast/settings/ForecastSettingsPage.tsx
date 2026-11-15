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
import { ErrorState } from '@/components/ui/error-state'
import { NumberMaskInput } from '@/components/ui/number-mask-input'
import { ToggleField } from '@/components/ui/toggle-field'
import { authFieldViolations as fieldViolations, SettingsCard, SettingsNativeSelect as NativeSelect, WIDE_FIELD } from '@/sdk'
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
import { Field } from '@/components/ui/field'

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
      {query.isPending ? (
        <>
          <PageHeader title={t('settings.title')} description={t('settings.description')} />
          <PageLoader />
        </>
      ) : query.isError ? (
        <>
          <PageHeader title={t('settings.title')} description={t('settings.description')} />
          <ErrorState onRetry={() => void query.refetch()} />
        </>
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

  const disabled = !canEdit || isSubmitting

  return (
    <>
      <PageHeader
        title={t('settings.title')}
        description={t('settings.description')}
        actions={
          canEdit && (
            <Button type="submit" form={formId} disabled={isSubmitting || !isDirty}>
              {t('settings.save')}
            </Button>
          )
        }
      />
      <form id={formId} className="grid gap-6" onSubmit={(event) => void onSubmit(event)} noValidate>
        <UnsavedChangesGuard when={canEdit && isDirty} />
        <SettingsCard title={t('settings.method')} description={t('settings.methodDescription')} disabled={disabled}>
          <Field label={t('settings.methodLabel')} description={t(`methodHints.${method}`)}>
            {(control) => (
              <NativeSelect {...control} {...register('method')}>
                {FORECAST_METHODS.map((value) => (
                  <option key={value} value={value}>
                    {t(`methods.${value}`)}
                  </option>
                ))}
              </NativeSelect>
            )}
          </Field>
          <Field label={t('settings.horizon')} description={t('settings.horizonHint')}>
            {(control) => (
              <NativeSelect {...control} {...register('horizon')}>
                {Array.from({ length: MAX_HORIZON_MONTHS }, (_, index) => index + 1).map((value) => (
                  <option key={value} value={value}>
                    {t('page.months', { count: value })}
                  </option>
                ))}
              </NativeSelect>
            )}
          </Field>
        </SettingsCard>

        <SettingsCard title={t('settings.categories')} description={t('settings.categoriesDescription')} disabled={disabled}>
          <Controller
            control={control}
            name="categoriesEnabled"
            render={({ field }) => (
              <ToggleField
                className={WIDE_FIELD}
                type="switch"
                label={t('settings.categoriesEnabled')}
                description={t('settings.categoriesHint')}
                checked={field.value}
                onCheckedChange={field.onChange}
                disabled={disabled}
              />
            )}
          />
          <Controller
            control={control}
            name="bestCase"
            render={({ field }) => (
              <Field label={t('settings.bestCase')} error={errorText(errors.bestCase?.message)}>
                {(control) => (
                  <NumberMaskInput
                    {...control}
                    kind="integer"
                    maxLength={3}
                    className="w-32"
                    placeholder={t('placeholders.percent')}
                    value={String(field.value ?? '')}
                    onValueChange={field.onChange}
                    onBlur={field.onBlur}
                  />
                )}
              </Field>
            )}
          />
          <Controller
            control={control}
            name="commit"
            render={({ field }) => (
              <Field label={t('settings.commit')} error={errorText(errors.commit?.message)}>
                {(control) => (
                  <NumberMaskInput
                    {...control}
                    kind="integer"
                    maxLength={3}
                    className="w-32"
                    placeholder={t('placeholders.percent')}
                    value={String(field.value ?? '')}
                    onValueChange={field.onChange}
                    onBlur={field.onBlur}
                  />
                )}
              </Field>
            )}
          />
          {showRanges && (
            <ul className={`${WIDE_FIELD} grid gap-1 rounded-md border bg-muted/40 p-3 text-sm`} aria-label={t('settings.rangesLabel')}>
              <li>{t('settings.ranges.pipeline', { max: Math.max(0, Number(bestCase) - 1) })}</li>
              <li>{t('settings.ranges.best_case', { min: Number(bestCase), max: Math.max(0, Number(commit) - 1) })}</li>
              <li>{t('settings.ranges.commit', { min: Number(commit) })}</li>
            </ul>
          )}
        </SettingsCard>
      </form>
    </>
  )
}
