import { zodResolver } from '@hookform/resolvers/zod'
import { useId, useMemo } from 'react'
import { Controller, useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { z } from 'zod'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Field } from '@/components/ui/field'
import { NumberMaskInput } from '@/components/ui/number-mask-input'
import { positiveInteger } from './api'
import { serverFieldErrors } from './violations'
import { ToggleField } from '@/components/ui/toggle-field'

export interface NumberSetting {
  key: string
  label: string
  hint?: string
  min: number
  max: number
  value: number
}

export interface ToggleSetting {
  key: string
  label: string
  value: boolean
}

interface NumericSettingsPanelProps {
  id: string
  title: string
  description: string
  numbers: NumberSetting[]
  toggles?: ToggleSetting[]
  serverPrefix: string
  onSave: (numbers: Record<string, number>, toggles: Record<string, boolean>) => Promise<void>
}

type PanelValues = { numbers: Record<string, string>; toggles: Record<string, boolean> }

function initialValues(numbers: NumberSetting[], toggles: ToggleSetting[]): PanelValues {
  return {
    numbers: Object.fromEntries(numbers.map((setting) => [setting.key, String(setting.value)])),
    toggles: Object.fromEntries(toggles.map((setting) => [setting.key, setting.value])),
  }
}

export function NumericSettingsPanel({
  id,
  title,
  description,
  numbers,
  toggles = [],
  serverPrefix,
  onSave,
}: NumericSettingsPanelProps) {
  const { t } = useTranslation('settings')
  const schema = useMemo(
    () =>
      z.object({
        numbers: z.object(
          Object.fromEntries(
            numbers.map((setting) => [
              setting.key,
              z
                .string()
                .refine((value) => positiveInteger(value, setting.min, setting.max), {
                  message: t('configuration.errors.range', { min: setting.min, max: setting.max }),
                }),
            ]),
          ),
        ),
        toggles: z.record(z.string(), z.boolean()),
      }),
    [numbers, t],
  )
  const {
    control,
    handleSubmit,
    reset,
    setError,
    formState: { errors, isSubmitting, isDirty },
  } = useForm<PanelValues>({ resolver: zodResolver(schema), defaultValues: initialValues(numbers, toggles) })

  const onSubmit = handleSubmit(async (values) => {
    try {
      const parsed = Object.fromEntries(Object.entries(values.numbers).map(([key, value]) => [key, Number(value)]))
      await onSave(parsed, values.toggles)
      reset(values)
      notifySuccess(t('configuration.saved'))
    } catch (error) {
      const fieldErrors = serverFieldErrors(
        error,
        Object.fromEntries(numbers.map((setting) => [`${serverPrefix}.${setting.key}`, setting.key])),
      )
      for (const [field, message] of fieldErrors) setError(`numbers.${field}`, { message })
      if (fieldErrors.length === 0) notifyError(error)
    }
  })

  const formId = useId()

  return (
    <Card>
      <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="grid gap-1.5">
          <CardTitle>{title}</CardTitle>
          <CardDescription>{description}</CardDescription>
        </div>
        <Button type="submit" form={formId} size="sm" disabled={isSubmitting || !isDirty}>
          {isSubmitting ? t('configuration.saving') : t('configuration.save')}
        </Button>
      </CardHeader>
      <CardContent>
        <form id={formId} className="grid gap-4 sm:grid-cols-2" onSubmit={onSubmit} noValidate>
          {numbers.map((setting) => (
            <Controller
              key={setting.key}
              control={control}
              name={`numbers.${setting.key}`}
              render={({ field }) => (
                <Field
                  id={`${id}-${setting.key}`}
                  label={setting.label}
                  description={setting.hint}
                  error={errors.numbers?.[setting.key]?.message}
                >
                  {(fieldControl) => (
                    <NumberMaskInput
                      {...fieldControl}
                      kind="integer"
                      maxLength={String(setting.max).length}
                      className="w-32"
                      placeholder={t('configuration.numeric.valuePlaceholder')}
                      value={field.value ?? ''}
                      onValueChange={field.onChange}
                      onBlur={field.onBlur}
                      disabled={isSubmitting}
                    />
                  )}
                </Field>
              )}
            />
          ))}
          {toggles.map((setting) => (
            <Controller
              key={setting.key}
              control={control}
              name={`toggles.${setting.key}`}
              render={({ field }) => (
                <ToggleField
                  className="sm:col-span-2"
                  type="switch"
                  label={setting.label}
                  checked={field.value}
                  onCheckedChange={field.onChange}
                  disabled={isSubmitting}
                />
              )}
            />
          ))}
        </form>
      </CardContent>
    </Card>
  )
}
