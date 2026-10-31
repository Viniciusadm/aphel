import { zodResolver } from '@hookform/resolvers/zod'
import { useForm, useWatch } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { z } from 'zod'
import { notifyError, notifySuccess, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { MailNotConfiguredNotice } from '../email/MailNotConfiguredNotice'
import { FormRow } from '../organization/components/FormRow'
import { positiveInteger, useSaveConfiguration, type OrganizationConfiguration } from './api'
import { ConfigurationGate } from './ConfigurationGate'
import { serverFieldErrors } from './violations'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

const schema = z.object({
  publicBaseUrl: z
    .string()
    .trim()
    .regex(/^https?:\/\/[^\s/?#@]+(\/[^\s?#]*)?$/, 'configuration.errors.url'),
  maxUploadMb: z.string().refine((value) => positiveInteger(value, 1, 4096), 'configuration.errors.uploadRange'),
})

type FormValues = z.infer<typeof schema>

function toFormValues(configuration: OrganizationConfiguration): FormValues {
  return {
    publicBaseUrl: configuration.public_base_url ?? configuration.effective_public_base_url,
    maxUploadMb: String(configuration.max_upload_mb),
  }
}

export function GeneralSettingsPage() {
  const { t } = useTranslation('settings')
  return (
    <Page aria-labelledby="general-settings-title">
<PageHeader title={t('configuration.general.title')} description={t('configuration.general.description')} />
      <MailNotConfiguredNotice />
      <ConfigurationGate>{(configuration) => <GeneralForm configuration={configuration} />}</ConfigurationGate>
    </Page>
  )
}

function GeneralForm({ configuration }: { configuration: OrganizationConfiguration }) {
  const { t } = useTranslation('settings')
  const save = useSaveConfiguration()
  const {
    register,
    control,
    handleSubmit,
    reset,
    setError,
    formState: { errors, isSubmitting, isDirty },
  } = useForm<FormValues>({ resolver: zodResolver(schema), defaultValues: toFormValues(configuration) })
  const publicBaseUrl = useWatch({ control, name: 'publicBaseUrl' })
  const insecure = publicBaseUrl.trim().startsWith('http://')

  const onSubmit = handleSubmit(async (values) => {
    try {
      const saved = await save({
        public_base_url: values.publicBaseUrl.trim().replace(/\/+$/, ''),
        max_upload_mb: Number(values.maxUploadMb),
      })
      reset(toFormValues(saved))
      notifySuccess(t('configuration.saved'))
    } catch (error) {
      const fieldErrors = serverFieldErrors(error, {
        public_base_url: 'publicBaseUrl',
        max_upload_mb: 'maxUploadMb',
      })
      for (const [field, message] of fieldErrors) setError(field as keyof FormValues, { message })
      if (fieldErrors.length === 0) notifyError(error)
    }
  })

  const errorText = (message?: string) =>
    message ? (message.startsWith('configuration.') ? t(message as 'configuration.saved') : message) : undefined

  return (
    <form className="grid max-w-2xl gap-6" onSubmit={onSubmit} noValidate>
      <UnsavedChangesGuard when={isDirty} />
      <fieldset className="grid gap-4" disabled={isSubmitting}>
        <FormRow
          id="general-public-base-url"
          label={t('configuration.general.publicBaseUrl')}
          error={errorText(errors.publicBaseUrl?.message)}
          hint={t('configuration.general.publicBaseUrlHint')}
        >
          {(describedBy) => (
            <Input placeholder={t('configuration.general.publicBaseUrlPlaceholder')}
              id="general-public-base-url"
              type="url"
              inputMode="url"
              autoComplete="url"
              aria-invalid={errors.publicBaseUrl ? true : undefined}
              aria-describedby={describedBy}
              {...register('publicBaseUrl')}
            />
          )}
        </FormRow>
        {insecure && (
          <p className="rounded-md border border-warning/40 bg-warning/10 p-3 text-sm">
            {t('configuration.general.insecureWarning')}
          </p>
        )}
        <FormRow
          id="general-max-upload"
          label={t('configuration.general.maxUploadMb')}
          error={errorText(errors.maxUploadMb?.message)}
          hint={t('configuration.general.maxUploadMbHint')}
        >
          {(describedBy) => (
            <Input placeholder={t('configuration.general.maxUploadPlaceholder')}
              id="general-max-upload"
              inputMode="numeric"
              className="w-32"
              aria-invalid={errors.maxUploadMb ? true : undefined}
              aria-describedby={describedBy}
              {...register('maxUploadMb')}
            />
          )}
        </FormRow>
      </fieldset>
      <div className="flex justify-end">
        <Button type="submit" disabled={isSubmitting || !isDirty}>
          {isSubmitting ? t('configuration.saving') : t('configuration.save')}
        </Button>
      </div>
    </form>
  )
}
