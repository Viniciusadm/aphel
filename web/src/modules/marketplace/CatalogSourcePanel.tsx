import { zodResolver } from '@hookform/resolvers/zod'
import { Controller, useForm, useWatch } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { z } from 'zod'
import { useSession } from '@/app/session/session-context'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { type CatalogSource, FormRow, positiveInteger, serverFieldErrors, SETTINGS_MANAGE, SettingsNativeSelect as NativeSelect, useOrganizationConfiguration, useSaveConfiguration } from '@/sdk'
import { ToggleField } from '@/components/ui/toggle-field'

const schema = z
  .object({
    mode: z.enum(['builtin', 'remote', 'off']),
    indexUrl: z.string().trim(),
    allowHttp: z.boolean(),
    timeoutSecs: z.string().refine((value) => positiveInteger(value, 1, 120), 'configuration.errors.timeout'),
  })
  .superRefine((values, context) => {
    if (values.mode !== 'remote') return
    const pattern = values.allowHttp ? /^https?:\/\/\S+$/ : /^https:\/\/\S+$/
    if (!pattern.test(values.indexUrl)) {
      context.addIssue({ code: 'custom', path: ['indexUrl'], message: 'configuration.errors.catalogUrl' })
    }
  })

type FormValues = z.infer<typeof schema>

function toFormValues(source: CatalogSource): FormValues {
  return {
    mode: source.mode,
    indexUrl: source.index_url ?? '',
    allowHttp: source.allow_http,
    timeoutSecs: String(source.timeout_secs),
  }
}

export function CatalogSourcePanel() {
  const { hasPermission } = useSession()
  const canManage = hasPermission(SETTINGS_MANAGE)
  const configuration = useOrganizationConfiguration(canManage)
  if (!canManage || !configuration.data) return null
  return <CatalogSourceForm key={JSON.stringify(configuration.data.catalog)} source={configuration.data.catalog} />
}

function CatalogSourceForm({ source }: { source: CatalogSource }) {
  const { t } = useTranslation('settings')
  const save = useSaveConfiguration()
  const {
    register,
    control,
    handleSubmit,
    reset,
    setError,
    formState: { errors, isSubmitting, isDirty },
  } = useForm<FormValues>({ resolver: zodResolver(schema), defaultValues: toFormValues(source) })
  const mode = useWatch({ control, name: 'mode' })
  const errorText = (message?: string) =>
    message ? (message.startsWith('configuration.') ? t(message as 'configuration.saved') : message) : undefined

  const onSubmit = handleSubmit(async (values) => {
    try {
      const saved = await save({
        catalog: {
          mode: values.mode,
          index_url: values.indexUrl === '' ? null : values.indexUrl,
          allow_http: values.allowHttp,
          timeout_secs: Number(values.timeoutSecs),
        },
      })
      reset(toFormValues(saved.catalog))
      notifySuccess(t('configuration.saved'))
    } catch (error) {
      const fieldErrors = serverFieldErrors(error, {
        'catalog.index_url': 'indexUrl',
        'catalog.timeout_secs': 'timeoutSecs',
      })
      for (const [field, message] of fieldErrors) setError(field as keyof FormValues, { message })
      if (fieldErrors.length === 0) notifyError(error)
    }
  })

  return (
    <Card className="max-w-2xl">
      <CardHeader>
        <CardTitle>{t('configuration.catalog.title')}</CardTitle>
        <CardDescription>{t('configuration.catalog.description')}</CardDescription>
      </CardHeader>
      <CardContent>
        <form className="grid gap-4" onSubmit={onSubmit} noValidate>
          <fieldset className="grid gap-4" disabled={isSubmitting}>
            <FormRow id="catalog-mode" label={t('configuration.catalog.mode')}>
              {(describedBy) => (
                <NativeSelect id="catalog-mode" aria-describedby={describedBy} {...register('mode')}>
                  <option value="builtin">{t('configuration.catalog.modes.builtin')}</option>
                  <option value="remote">{t('configuration.catalog.modes.remote')}</option>
                  <option value="off">{t('configuration.catalog.modes.off')}</option>
                </NativeSelect>
              )}
            </FormRow>
            {mode === 'remote' && (
              <>
                <FormRow
                  id="catalog-index-url"
                  label={t('configuration.catalog.indexUrl')}
                  error={errorText(errors.indexUrl?.message)}
                >
                  {(describedBy) => (
                    <Input
                      id="catalog-index-url"
                      type="url"
                      placeholder="https://example.com/catalog/index.json"
                      aria-invalid={errors.indexUrl ? true : undefined}
                      aria-describedby={describedBy}
                      {...register('indexUrl')}
                    />
                  )}
                </FormRow>
                <Controller
                  control={control}
                  name="allowHttp"
                  render={({ field }) => (
                    <ToggleField label={t('configuration.catalog.allowHttp')} checked={field.value} onCheckedChange={(checked) => field.onChange(checked === true)} />
                  )}
                />
              </>
            )}
            {mode !== 'off' && (
              <FormRow
                id="catalog-timeout"
                label={t('configuration.catalog.timeout')}
                error={errorText(errors.timeoutSecs?.message)}
              >
                {(describedBy) => (
                  <Input placeholder={t('modules.catalog.timeoutPlaceholder')}
                    id="catalog-timeout"
                    inputMode="numeric"
                    className="w-32"
                    aria-invalid={errors.timeoutSecs ? true : undefined}
                    aria-describedby={describedBy}
                    {...register('timeoutSecs')}
                  />
                )}
              </FormRow>
            )}
          </fieldset>
          <div className="flex justify-end">
            <Button type="submit" disabled={isSubmitting || !isDirty}>
              {isSubmitting ? t('configuration.saving') : t('configuration.save')}
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  )
}
