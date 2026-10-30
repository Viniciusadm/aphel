import { zodResolver } from '@hookform/resolvers/zod'
import { useEffect, useId, useMemo, useState } from 'react'
import { Controller, useForm, type Control } from 'react-hook-form'
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
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'
import { modulesApi, type ModuleInfo, type ModuleSettingField } from '@/kernel/modules/api'
import { moduleDefinition } from '@/kernel/modules/registry'
import { useFieldSchema } from '@/features/records/api'
import { activeFields } from '@/features/records/fields'
import {
  buildFormSchema,
  initialValues,
  isSecretConfigured,
  serverViolations,
  toSettings,
  violationField,
  violationKey,
  type SettingViolation,
  type SettingsFormValues,
  type ValidationKey,
} from './form'
import { useModuleText } from './text'

interface ModuleSettingsDialogProps {
  module: ModuleInfo
  initialViolations: SettingViolation[]
  onClose: () => void
  onSaved: () => Promise<unknown>
}

export function ModuleSettingsDialog({ module, initialViolations, onClose, onSaved }: ModuleSettingsDialogProps) {
  const { t } = useTranslation('modules')
  const text = useModuleText(module)
  const formId = useId()
  const [clearedSecrets, setClearedSecrets] = useState<string[]>([])
  const [formError, setFormError] = useState<string | null>(null)
  const schema = useMemo(() => buildFormSchema(module, clearedSecrets), [module, clearedSecrets])
  const fields = module.settings_schema
  const SettingsPanel = moduleDefinition(module.key)?.settingsPanel
  const {
    control,
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<SettingsFormValues, unknown, SettingsFormValues>({
    resolver: zodResolver(schema),
    defaultValues: initialValues(module),
  })

  useEffect(() => {
    let focused = false
    for (const violation of initialViolations) {
      const key = violationField(violation)
      if (!fields.some((field) => field.key === key)) continue
      setError(key, { type: 'server', message: violationKey(violation) }, { shouldFocus: !focused })
      focused = true
    }
  }, [initialViolations, fields, setError])

  const onSubmit = handleSubmit(async (values) => {
    setFormError(null)
    try {
      await modulesApi.updateSettings(module.key, toSettings(module, values, clearedSecrets))
      notifySuccess(t('toast.saved', { name: text.name }))
      await onSaved()
      onClose()
    } catch (error) {
      let mapped = false
      for (const violation of serverViolations(error)) {
        const key = violationField(violation)
        if (fields.some((field) => field.key === key)) {
          setError(key, { type: 'server', message: violationKey(violation) }, { shouldFocus: !mapped })
          mapped = true
        }
      }
      if (mapped) return
      setFormError(isApiError(error) && error.status === 403 ? t('errors.actionFailed') : t('errors.generic'))
    }
  })

  const errorText = (field: ModuleSettingField): string | null => {
    const message = errors[field.key]?.message as ValidationKey | undefined
    if (!message) return null
    return t(message, { min: field.min ?? '-∞', max: field.max ?? '∞' })
  }

  const toggleClear = (key: string, checked: boolean) =>
    setClearedSecrets((current) => (checked ? [...current, key] : current.filter((item) => item !== key)))

  return (
    <Dialog open onOpenChange={(open) => !open && !isSubmitting && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('settings.title', { name: text.name })}</DialogTitle>
          <DialogDescription>{t('settings.description')}</DialogDescription>
        </DialogHeader>
        {initialViolations.length > 0 && (
          <p role="status" className="rounded-md border border-warning/40 bg-warning/10 px-3 py-2 text-sm">
            {t('settings.requiredHint')}
          </p>
        )}
        <form id={formId} className="grid gap-4" noValidate onSubmit={onSubmit}>
          {formError && (
            <p role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
              {formError}
            </p>
          )}
          {fields.length === 0 && <p className="text-sm text-muted-foreground">{t('settings.noFields')}</p>}
          {fields.map((field) => {
            const inputId = `${formId}-${field.key}`
            const errorId = `${inputId}-error`
            const helpId = `${inputId}-help`
            const message = errorText(field)
            const help = text.fieldHelp(field)
            const describedBy = [message ? errorId : null, help ? helpId : null].filter(Boolean).join(' ') || undefined
            const label = `${text.fieldLabel(field)}${field.required ? ' *' : ''}`
            const secretConfigured = field.type === 'secret' && isSecretConfigured(module, field.key)
            const clearing = clearedSecrets.includes(field.key)
            const invalid = message ? true : undefined

            const footer = (
              <>
                {help && (
                  <p id={helpId} className="text-xs text-muted-foreground">
                    {help}
                  </p>
                )}
                {message && (
                  <p id={errorId} className="text-xs text-destructive">
                    {message}
                  </p>
                )}
              </>
            )

            if (field.type === 'boolean') {
              return (
                <div key={field.key} className="grid gap-1">
                  <Controller
                    control={control}
                    name={field.key}
                    render={({ field: control }) => (
                      <label htmlFor={inputId} className="flex items-center gap-2 text-sm">
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
                  {footer}
                </div>
              )
            }

            return (
              <div key={field.key} className="grid gap-1.5">
                <Label htmlFor={inputId}>{label}</Label>
                {field.type === 'select' ? (
                  <Controller
                    control={control}
                    name={field.key}
                    render={({ field: control }) => (
                      <Select value={typeof control.value === 'string' ? control.value : ''} onValueChange={control.onChange}>
                        <SelectTrigger id={inputId} className="w-full" aria-invalid={invalid} aria-describedby={describedBy}>
                          <SelectValue placeholder={t('settings.selectPlaceholder')} />
                        </SelectTrigger>
                        <SelectContent>
                          {field.options.map((option) => (
                            <SelectItem key={option.value} value={option.value}>
                              {text.optionLabel(field, option.value, option.label)}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    )}
                  />
                ) : field.type === 'field' ? (
                  <FieldPicker
                    control={control}
                    field={field}
                    inputId={inputId}
                    invalid={invalid}
                    describedBy={describedBy}
                  />
                ) : field.type === 'long_text' ? (
                  <Textarea id={inputId} rows={4} aria-invalid={invalid} aria-describedby={describedBy} {...register(field.key)} />
                ) : (
                  <Input
                    id={inputId}
                    type={inputType(field)}
                    inputMode={field.type === 'number' ? 'decimal' : undefined}
                    min={field.type === 'number' ? (field.min ?? undefined) : undefined}
                    max={field.type === 'number' ? (field.max ?? undefined) : undefined}
                    autoComplete={field.type === 'secret' ? 'new-password' : 'off'}
                    disabled={clearing}
                    placeholder={secretConfigured ? '••••••••' : undefined}
                    aria-invalid={invalid}
                    aria-describedby={describedBy}
                    {...register(field.key)}
                  />
                )}
                {secretConfigured && <p className="text-xs text-muted-foreground">{t('settings.secretConfigured')}</p>}
                {secretConfigured && !field.required && (
                  <label className="flex items-center gap-2 text-xs">
                    <Checkbox checked={clearing} onCheckedChange={(checked) => toggleClear(field.key, checked === true)} />
                    {t('settings.clearSecret')}
                  </label>
                )}
                {footer}
              </div>
            )
          })}
        </form>
        {SettingsPanel && (
          <section className="grid gap-2 border-t pt-4" aria-label={t('settings.extraPanel')}>
            <h3 className="text-sm font-semibold">{t('settings.extraPanel')}</h3>
            {module.enabled ? (
              <SettingsPanel moduleKey={module.key} />
            ) : (
              <p className="text-sm text-muted-foreground">{t('settings.extraPanelDisabled')}</p>
            )}
          </section>
        )}
        <DialogFooter>
          <Button type="button" variant="outline" disabled={isSubmitting} onClick={onClose}>
            {t('settings.cancel')}
          </Button>
          {fields.length > 0 && (
            <Button type="submit" form={formId} disabled={isSubmitting} aria-busy={isSubmitting}>
              {t('settings.save')}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

interface FieldPickerProps {
  control: Control<SettingsFormValues, unknown, SettingsFormValues>
  field: ModuleSettingField
  inputId: string
  invalid: true | undefined
  describedBy: string | undefined
}

function FieldPicker({ control, field, inputId, invalid, describedBy }: FieldPickerProps) {
  const { t } = useTranslation('modules')
  const schema = useFieldSchema(field.record_type ?? '')
  const options = useMemo(() => {
    if (!schema.data) return []
    return activeFields(schema.data).filter(
      (candidate) => field.field_types.length === 0 || field.field_types.includes(candidate.field_type),
    )
  }, [schema.data, field.field_types])

  return (
    <Controller
      control={control}
      name={field.key}
      render={({ field: control }) => {
        const value = typeof control.value === 'string' ? control.value : ''
        const known = value === '' || options.some((option) => option.key === value)
        return (
          <Select value={value} onValueChange={control.onChange} disabled={schema.isPending || !field.record_type}>
            <SelectTrigger id={inputId} className="w-full" aria-invalid={invalid} aria-describedby={describedBy} aria-busy={schema.isPending}>
              <SelectValue placeholder={t('settings.fieldPlaceholder')} />
            </SelectTrigger>
            <SelectContent>
              {!known && <SelectItem value={value}>{value}</SelectItem>}
              {options.map((option) => (
                <SelectItem key={option.key} value={option.key}>
                  {option.label}
                </SelectItem>
              ))}
              {options.length === 0 && known && (
                <p className="px-2 py-1.5 text-sm text-muted-foreground">{t('settings.noFieldOptions')}</p>
              )}
            </SelectContent>
          </Select>
        )
      }}
    />
  )
}

function inputType(field: ModuleSettingField): string {
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
