import { NativeSelect } from '@/components/ui/native-select'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { zodResolver } from '@hookform/resolvers/zod'
import { useId, useMemo, useState } from 'react'
import { Controller, useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { integrationsApi, isSecretConfigured, type Integration, type IntegrationField } from './api'
import {
  buildFormSchema,
  initialValues,
  serverViolations,
  toConfig,
  violationField,
  violationKey,
  type ConfigFormValues,
  type ValidationKey,
} from './form'
import { useIntegrationText } from './text'
import { Alert } from '@/components/ui/alert'
import { Field } from '@/components/ui/field'

interface IntegrationConfigDialogProps {
  integration: Integration
  open: boolean
  onOpenChange: (open: boolean) => void
  onSaved: () => void
}

export function IntegrationConfigDialog({ integration, open, onOpenChange, onSaved }: IntegrationConfigDialogProps) {
  const { t } = useTranslation('integration-card')
  const text = useIntegrationText()
  const formId = useId()
  const schema = useMemo(() => buildFormSchema(integration), [integration])
  const [clearSecrets, setClearSecrets] = useState<string[]>([])
  const [formError, setFormError] = useState<string | null>(null)
  const {
    control,
    register,
    handleSubmit,
    reset,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ConfigFormValues, unknown, ConfigFormValues>({
    resolver: zodResolver(schema),
    defaultValues: initialValues(integration),
  })

  const close = (next: boolean) => {
    if (!next) {
      reset(initialValues(integration))
      setClearSecrets([])
      setFormError(null)
    }
    onOpenChange(next)
  }

  const onSubmit = handleSubmit(async (values) => {
    setFormError(null)
    try {
      await integrationsApi.configure(
        integration.provider_key,
        toConfig(integration, values),
        clearSecrets,
        integration.updated_at ?? null,
      )
      notifySuccess(t('toast.saved', { name: text.name(integration) }))
      onSaved()
      close(false)
    } catch (error) {
      let mapped = false
      for (const violation of serverViolations(error)) {
        const field = violationField(violation)
        if (field && integration.fields.some((item) => item.key === field)) {
          setError(field, { type: 'server', message: violationKey(violation) }, { shouldFocus: !mapped })
          mapped = true
        }
      }
      if (mapped) return
      if (isApiError(error) && error.code === 'version_conflict') setFormError(t('errors.versionConflict'))
      else if (isApiError(error) && error.code === 'configuration_error') setFormError(t('errors.encryptionMissing'))
      else setFormError(t('errors.generic'))
    }
  })

  const errorText = (field: IntegrationField): string | null => {
    const message = errors[field.key]?.message as ValidationKey | undefined
    if (!message) return null
    return t(message, { min: field.min ?? '', max: field.max ?? '' })
  }

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('form.title', { name: text.name(integration) })}</DialogTitle>
          <DialogDescription>{t('form.description')}</DialogDescription>
        </DialogHeader>
        <form id={formId} className="grid gap-4" noValidate onSubmit={onSubmit}>
          {formError && (
            <Alert className="text-destructive">
              {formError}
            </Alert>
          )}
          {integration.fields.map((field) => {
            const inputId = `${formId}-${field.key}`
            const errorId = `${inputId}-error`
            const helpId = `${inputId}-help`
            const message = errorText(field)
            const help = text.fieldHelp(integration, field)
            const describedBy = [message ? errorId : null, help ? helpId : null].filter(Boolean).join(' ') || undefined
            const label = `${text.fieldLabel(integration, field)}${field.required ? ' *' : ''}`
            const secretConfigured = field.type === 'secret' && isSecretConfigured(integration, field.key)
            const clearing = clearSecrets.includes(field.key)

            if (field.type === 'boolean') {
              return (
                <div key={field.key} className="grid gap-1">
                  <Controller
                    control={control}
                    name={field.key}
                    render={({ field: control }) => (
                      <label className="flex items-center gap-2 text-sm">
                        <Checkbox
                          id={inputId}
                          checked={control.value === true}
                          aria-describedby={describedBy}
                          onCheckedChange={(checked) => control.onChange(checked === true)}
                        />
                        {label}
                      </label>
                    )}
                  />
                  {help && (
                    <p id={helpId} className="text-xs text-muted-foreground">
                      {help}
                    </p>
                  )}
                </div>
              )
            }

            return (
              <Field key={field.key} id={inputId} label={label}>
                {field.type === 'select' ? (
                  <Controller
                    control={control}
                    name={field.key}
                    render={({ field: control }) => (
                      <NativeSelect value={typeof control.value === 'string' ? control.value : ''} onValueChange={control.onChange} id={inputId} aria-invalid={message ? true : undefined} aria-describedby={describedBy}><option value="" disabled>{t('form.selectPlaceholder')}</option>
                          {field.options.map((option) => (
                            <option key={option} value={option}>
                              {text.optionLabel(integration, field, option)}
                            </option>
                          ))}
                        </NativeSelect>
                    )}
                  />
                ) : (
                  <Input
                    id={inputId}
                    type={inputType(field)}
                    inputMode={field.type === 'number' ? 'numeric' : undefined}
                    autoComplete={field.type === 'secret' ? 'new-password' : 'off'}
                    disabled={clearing}
                    placeholder={secretConfigured ? '••••••••' : undefined}
                    aria-invalid={message ? true : undefined}
                    aria-describedby={describedBy}
                    {...register(field.key)}
                  />
                )}
                {secretConfigured && (
                  <p className="text-xs text-muted-foreground">{t('form.secretConfigured')}</p>
                )}
                {secretConfigured && !field.required && (
                  <label className="flex items-center gap-2 text-xs">
                    <Checkbox
                      checked={clearing}
                      onCheckedChange={(checked) =>
                        setClearSecrets((current) =>
                          checked === true ? [...current, field.key] : current.filter((key) => key !== field.key),
                        )
                      }
                    />
                    {t('form.clearSecret')}
                  </label>
                )}
                {help && (
                  <p id={helpId} className="text-xs text-muted-foreground">
                    {help}
                  </p>
                )}
                {message && (
                  <SharedFieldError id={errorId}>
                    {message}
                  </SharedFieldError>
                )}
              </Field>
            )
          })}
        </form>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => close(false)}>
            {t('form.cancel')}
          </Button>
          <Button type="submit" form={formId} disabled={isSubmitting}>
            {t('form.save')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function inputType(field: IntegrationField): string {
  switch (field.type) {
    case 'secret':
      return 'password'
    case 'email':
      return 'email'
    case 'url':
      return 'url'
    case 'number':
      return 'number'
    default:
      return 'text'
  }
}
