import { zodResolver } from '@hookform/resolvers/zod'
import { useState } from 'react'
import { Controller, useForm, useWatch, type Control } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { z } from 'zod'
import { notifyError, notifySuccess, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  testStorage,
  useSaveConfiguration,
  type OrganizationConfiguration,
  type StorageSettings,
  type StorageTestResult,
} from '../general/api'
import { ConfigurationGate } from '../general/ConfigurationGate'
import { serverFieldErrors } from '../general/violations'
import { FormRow } from '../organization/components/FormRow'
import { NativeSelect } from '../organization/components/NativeSelect'
import { PageHeader } from '@/components/ui/page-header'
import { SettingsCard, WIDE_FIELD } from '../components/SettingsCard'
import { Page } from '@/components/ui/page'
import { ToggleField } from '@/components/ui/toggle-field'

const schema = z
  .object({
    driver: z.enum(['local', 's3']),
    path: z.string().trim(),
    bucket: z.string().trim(),
    region: z.string().trim(),
    endpoint: z.string().trim(),
    accessKeyId: z.string().trim(),
    secretAccessKey: z.string(),
    clearSecret: z.boolean(),
    allowHttp: z.boolean(),
    forcePathStyle: z.boolean(),
  })
  .superRefine((values, context) => {
    const required = (field: 'path' | 'bucket' | 'region') =>
      context.addIssue({ code: 'custom', path: [field], message: 'configuration.errors.required' })
    if (values.driver === 'local' && values.path === '') required('path')
    if (values.driver === 's3') {
      if (values.bucket === '') required('bucket')
      if (values.region === '') required('region')
      if (values.endpoint !== '' && !/^https?:\/\/\S+$/.test(values.endpoint)) {
        context.addIssue({ code: 'custom', path: ['endpoint'], message: 'configuration.errors.url' })
      }
    }
  })

type FormValues = z.infer<typeof schema>

function toFormValues(configuration: OrganizationConfiguration): FormValues {
  const storage = configuration.storage
  return {
    driver: storage.driver,
    path: storage.path ?? './storage',
    bucket: storage.bucket ?? 'aphel',
    region: storage.region ?? 'us-east-1',
    endpoint: storage.endpoint ?? '',
    accessKeyId: storage.access_key_id ?? '',
    secretAccessKey: '',
    clearSecret: false,
    allowHttp: storage.allow_http,
    forcePathStyle: storage.force_path_style,
  }
}

function toSettings(values: FormValues): StorageSettings {
  const optional = (value: string) => (value === '' ? null : value)
  return values.driver === 'local'
    ? { driver: 'local', path: values.path, allow_http: false, force_path_style: true }
    : {
        driver: 's3',
        bucket: values.bucket,
        region: values.region,
        endpoint: optional(values.endpoint),
        access_key_id: optional(values.accessKeyId),
        allow_http: values.allowHttp,
        force_path_style: values.forcePathStyle,
      }
}

export function StorageSettingsPage() {
  const { t } = useTranslation('settings')
  return (
    <ConfigurationGate
      fallback={(content) => (
        <Page>
          <PageHeader title={t('configuration.storage.title')} description={t('configuration.storage.description')} />
          {content}
        </Page>
      )}
    >
      {(configuration) => <StorageForm configuration={configuration} />}
    </ConfigurationGate>
  )
}

function StorageForm({ configuration }: { configuration: OrganizationConfiguration }) {
  const { t } = useTranslation('settings')
  const save = useSaveConfiguration()
  const [testing, setTesting] = useState(false)
  const [result, setResult] = useState<StorageTestResult | null>(null)
  const {
    register,
    control,
    handleSubmit,
    reset,
    setError,
    getValues,
    trigger,
    formState: { errors, isSubmitting, isDirty },
  } = useForm<FormValues>({ resolver: zodResolver(schema), defaultValues: toFormValues(configuration) })
  const driver = useWatch({ control, name: 'driver' })
  const driverChanged = driver !== configuration.storage.driver

  const errorText = (message?: string) =>
    message ? (message.startsWith('configuration.') ? t(message as 'configuration.saved') : message) : undefined

  const onSubmit = handleSubmit(async (values) => {
    try {
      const secret =
        values.driver !== 's3'
          ? undefined
          : values.clearSecret
            ? null
            : values.secretAccessKey === ''
              ? undefined
              : values.secretAccessKey
      const saved = await save({
        storage: toSettings(values),
        ...(secret === undefined ? {} : { storage_secret_access_key: secret }),
      })
      reset(toFormValues(saved))
      setResult(null)
      notifySuccess(t('configuration.saved'))
    } catch (error) {
      const fieldErrors = serverFieldErrors(error, {
        'storage.path': 'path',
        'storage.bucket': 'bucket',
        'storage.region': 'region',
        'storage.endpoint': 'endpoint',
      })
      for (const [field, message] of fieldErrors) setError(field as keyof FormValues, { message })
      if (fieldErrors.length === 0) notifyError(error)
    }
  })

  const runTest = async () => {
    if (!(await trigger())) return
    const values = getValues()
    setTesting(true)
    setResult(null)
    try {
      setResult(
        await testStorage(
          toSettings(values),
          values.driver === 's3' && values.secretAccessKey !== '' ? values.secretAccessKey : null,
        ),
      )
    } catch (error) {
      notifyError(error)
    } finally {
      setTesting(false)
    }
  }

  const busy = isSubmitting || testing

  return (
    <Page>
      <PageHeader
        title={t('configuration.storage.title')}
        description={t('configuration.storage.description')}
        actions={
          <div className="flex flex-wrap gap-2">
            <Button type="button" variant="outline" disabled={busy} onClick={() => void runTest()}>
              {testing ? t('configuration.storage.testing') : t('configuration.storage.test')}
            </Button>
            <Button type="submit" form="storage-form" disabled={busy || !isDirty}>
              {isSubmitting ? t('configuration.saving') : t('configuration.save')}
            </Button>
          </div>
        }
      />
      {result && (
        <p
          role="status"
          className={
            result.ok
              ? 'rounded-md border border-success/40 bg-success/10 p-3 text-sm'
              : 'rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive'
          }
        >
          {result.ok
            ? t('configuration.storage.testOk')
            : t('configuration.storage.testFailed', { message: result.message ?? '' })}
        </p>
      )}
    <form id="storage-form" className="grid gap-6" onSubmit={onSubmit} noValidate>
      <UnsavedChangesGuard when={isDirty} />
      <SettingsCard title={t('configuration.storage.sections.location')} disabled={busy}>
        <FormRow id="storage-driver" label={t('configuration.storage.driver')}>
          {(describedBy) => (
            <NativeSelect id="storage-driver" aria-describedby={describedBy} {...register('driver')}>
              <option value="local">{t('configuration.storage.drivers.local')}</option>
              <option value="s3">{t('configuration.storage.drivers.s3')}</option>
            </NativeSelect>
          )}
        </FormRow>
        {driverChanged && (
          <p className={`rounded-md border border-warning/40 bg-warning/10 p-3 text-sm ${WIDE_FIELD}`}>
            {t('configuration.storage.driverChangeWarning')}
          </p>
        )}
        {driver === 'local' ? (
          <div className={WIDE_FIELD}>
          <FormRow
            id="storage-path"
            label={t('configuration.storage.path')}
            error={errorText(errors.path?.message)}
            hint={t('configuration.storage.pathHint')}
          >
            {(describedBy) => (
              <Input placeholder={t('configuration.storage.pathPlaceholder')}
                id="storage-path"
                className="font-mono"
                aria-invalid={errors.path ? true : undefined}
                aria-describedby={describedBy}
                {...register('path')}
              />
            )}
          </FormRow>
          </div>
        ) : (
          <>
            <FormRow id="storage-bucket" label={t('configuration.storage.bucket')} error={errorText(errors.bucket?.message)}>
              {(describedBy) => (
                <Input placeholder={t('configuration.storage.bucketPlaceholder')} id="storage-bucket" aria-describedby={describedBy} {...register('bucket')} />
              )}
            </FormRow>
            <FormRow id="storage-region" label={t('configuration.storage.region')} error={errorText(errors.region?.message)}>
              {(describedBy) => (
                <Input placeholder={t('configuration.storage.regionPlaceholder')} id="storage-region" aria-describedby={describedBy} {...register('region')} />
              )}
            </FormRow>
            <div className={WIDE_FIELD}>
            <FormRow
              id="storage-endpoint"
              label={t('configuration.storage.endpoint')}
              error={errorText(errors.endpoint?.message)}
              hint={t('configuration.storage.endpointHint')}
            >
              {(describedBy) => (
                <Input
                  id="storage-endpoint"
                  type="url"
                  placeholder="https://s3.example.com"
                  aria-describedby={describedBy}
                  {...register('endpoint')}
                />
              )}
            </FormRow>
            </div>
          </>
        )}
      </SettingsCard>
      {driver === 's3' && (
        <SettingsCard title={t('configuration.storage.sections.credentials')} disabled={busy}>
            <FormRow id="storage-access-key" label={t('configuration.storage.accessKeyId')}>
              {(describedBy) => (
                <Input placeholder={t('configuration.storage.accessKeyIdPlaceholder')}
                  id="storage-access-key"
                  autoComplete="off"
                  aria-describedby={describedBy}
                  {...register('accessKeyId')}
                />
              )}
            </FormRow>
            <FormRow
              id="storage-secret-key"
              label={t('configuration.storage.secretAccessKey')}
              hint={
                configuration.storage_secret_configured
                  ? t('configuration.storage.secretConfigured')
                  : t('configuration.storage.secretMissing')
              }
            >
              {(describedBy) => (
                <Input placeholder={t('configuration.storage.secretKeyPlaceholder')}
                  id="storage-secret-key"
                  type="password"
                  autoComplete="new-password"
                  aria-describedby={describedBy}
                  {...register('secretAccessKey')}
                />
              )}
            </FormRow>
            {configuration.storage_secret_configured && (
              <div className={WIDE_FIELD}>
                <CheckboxRow control={control} name="clearSecret" label={t('configuration.storage.clearSecret')} />
              </div>
            )}
            <div className={WIDE_FIELD}>
              <CheckboxRow control={control} name="forcePathStyle" label={t('configuration.storage.forcePathStyle')} />
            </div>
            <div className={WIDE_FIELD}>
              <CheckboxRow control={control} name="allowHttp" label={t('configuration.storage.allowHttp')} />
            </div>
        </SettingsCard>
      )}
    </form>
    </Page>
  )
}

interface CheckboxRowProps {
  control: Control<FormValues>
  name: 'clearSecret' | 'allowHttp' | 'forcePathStyle'
  label: string
}

function CheckboxRow({ control, name, label }: CheckboxRowProps) {
  return (
    <Controller
      control={control}
      name={name}
      render={({ field }) => (
        <ToggleField label={label} checked={field.value} onCheckedChange={(checked) => field.onChange(checked === true)} />
      )}
    />
  )
}
