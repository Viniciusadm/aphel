import { zodResolver } from '@hookform/resolvers/zod'
import { useState } from 'react'
import { useForm, useWatch } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { useLocation } from 'react-router'
import { z } from 'zod'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, notifyError, notifySuccess, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { MailNotConfiguredNotice } from '../email/MailNotConfiguredNotice'
import { FormRow } from '../organization/components/FormRow'
import { positiveInteger, useSaveConfiguration, type OrganizationConfiguration } from './api'
import { ConfigurationGate } from './ConfigurationGate'
import { serverFieldErrors } from './violations'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

const SYSTEM_MANAGE = 'system.manage'
const RESERVED_APP_SEGMENTS = ['api']
const RECOVERY_APP_PATH = '/_aphel/app/'

const schema = z.object({
  publicBaseUrl: z
    .string()
    .trim()
    .regex(/^https?:\/\/[^\s/?#@]+(\/[^\s?#]*)?$/, 'configuration.errors.url'),
  appSegment: z
    .string()
    .trim()
    .toLowerCase()
    .regex(/^([a-z0-9]([a-z0-9-]{0,38}[a-z0-9])?)?$/, 'configuration.errors.appPath')
    .refine((value) => !RESERVED_APP_SEGMENTS.includes(value), 'configuration.errors.appPathReserved'),
  siteSegment: z
    .string()
    .trim()
    .toLowerCase()
    .regex(/^([a-z0-9]([a-z0-9-]{0,38}[a-z0-9])?)?$/, 'configuration.errors.appPath')
    .refine((value) => !RESERVED_APP_SEGMENTS.includes(value), 'configuration.errors.appPathReserved'),
  siteHost: z
    .string()
    .trim()
    .toLowerCase()
    .regex(/^([a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)*(:\d{1,5})?)?$/, 'configuration.errors.siteHost'),
  maxUploadMb: z.string().refine((value) => positiveInteger(value, 1, 4096), 'configuration.errors.uploadRange'),
}).refine((values) => values.siteHost.trim() !== '' || values.siteSegment.trim().toLowerCase() !== values.appSegment.trim().toLowerCase(), {
  path: ['siteSegment'],
  message: 'configuration.errors.sitePathTaken',
})

type FormValues = z.infer<typeof schema>

function segmentOf(basePath: string): string {
  return basePath.replace(/^\/+|\/+$/g, '')
}

function basePathOf(segment: string): string {
  const trimmed = segment.trim().toLowerCase()
  return trimmed ? `/${trimmed}` : '/'
}

function toFormValues(configuration: OrganizationConfiguration): FormValues {
  return {
    publicBaseUrl: configuration.public_base_url ?? configuration.effective_public_base_url,
    appSegment: segmentOf(configuration.app_base_path),
    siteSegment: segmentOf(configuration.site_base_path),
    siteHost: configuration.site_host ?? '',
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
  const { hasPermission } = useSession()
  const location = useLocation()
  const canMoveApp = hasPermission(SYSTEM_MANAGE)
  const [pendingMove, setPendingMove] = useState<FormValues | null>(null)
  const {
    register,
    control,
    handleSubmit,
    reset,
    setError,
    formState: { errors, isSubmitting, isDirty },
  } = useForm<FormValues>({ resolver: zodResolver(schema), defaultValues: toFormValues(configuration) })
  const publicBaseUrl = useWatch({ control, name: 'publicBaseUrl' })
  const appSegment = useWatch({ control, name: 'appSegment' })
  const siteHost = useWatch({ control, name: 'siteHost' })
  const sitePrefix = siteHost.trim()
    ? `${publicBaseUrl.trim().startsWith('https://') ? 'https' : 'http'}://${siteHost.trim().toLowerCase().replace(/\/+$/, '')}/`
    : null
  const insecure = publicBaseUrl.trim().startsWith('http://')
  const addressPrefix = `${publicBaseUrl.trim().replace(/\/+$/, '')}/`
  const nextAppUrl = `${addressPrefix}${appSegment.trim().toLowerCase()}`.replace(/\/+$/, '/')

  const persist = async (values: FormValues) => {
    try {
      const saved = await save({
        public_base_url: values.publicBaseUrl.trim().replace(/\/+$/, ''),
        ...(canMoveApp ? { app_base_path: basePathOf(values.appSegment) } : {}),
        ...(canMoveApp && configuration.site_available
          ? { site_base_path: basePathOf(values.siteSegment), site_host: values.siteHost.trim().toLowerCase() || null }
          : {}),
        max_upload_mb: Number(values.maxUploadMb),
      })
      if (saved.app_base_path !== configuration.app_base_path) {
        window.location.assign(`${new URL(saved.app_base_url).pathname.replace(/\/+$/, '')}${location.pathname}`)
        return
      }
      reset(toFormValues(saved))
      notifySuccess(t('configuration.saved'))
    } catch (error) {
      const fieldErrors = serverFieldErrors(error, {
        public_base_url: 'publicBaseUrl',
        'mounts.app.base_path': 'appSegment',
        'mounts.site.base_path': 'siteSegment',
        'mounts.site.host': 'siteHost',
        max_upload_mb: 'maxUploadMb',
      })
      for (const [field, message] of fieldErrors) setError(field as keyof FormValues, { message })
      if (fieldErrors.length === 0) notifyError(error)
    }
  }

  const onSubmit = handleSubmit(async (values) => {
    if (canMoveApp && basePathOf(values.appSegment) !== configuration.app_base_path) {
      setPendingMove(values)
      return
    }
    await persist(values)
  })

  const errorText = (message?: string) =>
    message ? (message.startsWith('configuration.') ? t(message as 'configuration.saved') : message) : undefined

  return (
    <form className="grid max-w-2xl gap-6" onSubmit={onSubmit} noValidate>
      <UnsavedChangesGuard when={isDirty && pendingMove === null} />
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
          id="general-app-path"
          label={t('configuration.general.appPath')}
          error={errorText(errors.appSegment?.message)}
          hint={canMoveApp
            ? t('configuration.general.appPathHint', { recovery: RECOVERY_APP_PATH })
            : t('configuration.general.appPathLocked')}
        >
          {(describedBy) => (
            <div className="flex min-w-0 items-center rounded-md border border-input bg-muted/40 shadow-xs">
              <span className="truncate pl-3 text-sm text-muted-foreground">{addressPrefix}</span>
              <Input
                id="general-app-path"
                className="min-w-24 flex-1 border-0 shadow-none"
                placeholder={t('configuration.general.appPathPlaceholder')}
                autoComplete="off"
                spellCheck={false}
                readOnly={!canMoveApp}
                aria-invalid={errors.appSegment ? true : undefined}
                aria-describedby={describedBy}
                {...register('appSegment')}
              />
            </div>
          )}
        </FormRow>
        {configuration.site_available && (
          <>
            <FormRow
              id="general-site-host"
              label={t('configuration.general.siteHost')}
              error={errorText(errors.siteHost?.message)}
              hint={canMoveApp ? t('configuration.general.siteHostHint') : t('configuration.general.appPathLocked')}
            >
              {(describedBy) => (
                <Input
                  id="general-site-host"
                  placeholder={t('configuration.general.siteHostPlaceholder')}
                  autoComplete="off"
                  spellCheck={false}
                  inputMode="url"
                  readOnly={!canMoveApp}
                  aria-invalid={errors.siteHost ? true : undefined}
                  aria-describedby={describedBy}
                  {...register('siteHost')}
                />
              )}
            </FormRow>
            <FormRow
              id="general-site-path"
              label={t('configuration.general.sitePath')}
              error={errorText(errors.siteSegment?.message)}
              hint={canMoveApp ? t('configuration.general.sitePathHint') : t('configuration.general.appPathLocked')}
            >
              {(describedBy) => (
                <div className="flex min-w-0 items-center rounded-md border border-input bg-muted/40 shadow-xs">
                  <span className="truncate pl-3 text-sm text-muted-foreground">{sitePrefix ?? addressPrefix}</span>
                  <Input
                    id="general-site-path"
                    className="min-w-24 flex-1 border-0 shadow-none"
                    placeholder={t('configuration.general.appPathPlaceholder')}
                    autoComplete="off"
                    spellCheck={false}
                    readOnly={!canMoveApp}
                    aria-invalid={errors.siteSegment ? true : undefined}
                    aria-describedby={describedBy}
                    {...register('siteSegment')}
                  />
                </div>
              )}
            </FormRow>
          </>
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
      <ConfirmDialog
        open={pendingMove !== null}
        onOpenChange={(open) => !open && setPendingMove(null)}
        title={t('configuration.general.moveAppTitle')}
        description={t('configuration.general.moveAppDescription', { url: nextAppUrl, recovery: RECOVERY_APP_PATH })}
        confirmLabel={t('configuration.general.moveAppConfirm')}
        notifyOnError={false}
        onConfirm={async () => {
          if (pendingMove) await persist(pendingMove)
          setPendingMove(null)
        }}
      />
    </form>
  )
}
