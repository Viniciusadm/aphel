import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { zodResolver } from '@hookform/resolvers/zod'
import { useQueryClient } from '@tanstack/react-query'
import { ImageUp, Trash2 } from 'lucide-react'
import { useEffect, useMemo, useRef, useState, type ChangeEvent } from 'react'
import { useForm, useWatch } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { z } from 'zod'
import { isApiError } from '@/api'
import { useSession } from '@/app/session/session-context'
import { SESSION_QUERY_KEY } from '@/app/session/session-query'
import { notifyError, notifySuccess, PageLoader, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { fieldViolations } from '@/features/auth/lib/api-errors'
import { MailNotConfiguredNotice } from '../email/MailNotConfiguredNotice'
import { SETTINGS_MANAGE } from '../lib/permissions'
import {
  LOGO_MIME_TYPES,
  MAX_LOGO_BYTES,
  organizationKeys,
  organizationLogoUrl,
  updateOrganization,
  uploadLogo,
  useOrganizationSettings,
  type OrganizationSettings,
} from './api'
import { FormRow } from './components/FormRow'
import { NativeSelect } from './components/NativeSelect'
import { brandPalette, contrastRatio, formatRatio, isHexColor } from './lib/contrast'
import { LOCALES, currencyLabel, currencyOptions, timezoneOptions } from './lib/preferences'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

const DEFAULT_PICKER_COLOR = '#4f46e5'

const schema = z.object({
  name: z.string().trim().min(1, 'errors.nameRequired').max(140, 'errors.nameLength'),
  primaryColor: z
    .string()
    .trim()
    .refine((value) => value === '' || isHexColor(value), 'errors.color'),
  locale: z.string().min(1),
  timezone: z.string().min(1, 'errors.timezone'),
  currency: z.string().regex(/^[A-Z]{3}$/, 'errors.currency'),
})

type FormValues = z.infer<typeof schema>

const serverFields: Record<string, keyof FormValues> = {
  name: 'name',
  primary_color: 'primaryColor',
  locale: 'locale',
  timezone: 'timezone',
  currency: 'currency',
}

const serverMessages: Record<keyof FormValues, string> = {
  name: 'errors.nameLength',
  primaryColor: 'errors.color',
  locale: 'errors.locale',
  timezone: 'errors.timezone',
  currency: 'errors.currency',
}

function toFormValues(organization: OrganizationSettings): FormValues {
  return {
    name: organization.name,
    primaryColor: organization.primary_color ?? '',
    locale: organization.locale,
    timezone: organization.timezone,
    currency: organization.currency,
  }
}

export function OrganizationSettingsPage() {
  const { t } = useTranslation('organization-settings')
  const query = useOrganizationSettings()

  return (
    <Page aria-labelledby="organization-settings-title">
<PageHeader title={t('title')} description={t('description')} />
      <MailNotConfiguredNotice />
      {query.isPending ? (
        <PageLoader />
      ) : query.isError ? (
        <ErrorState title={t('loadError')} onRetry={() => void query.refetch()} />
      ) : (
        <OrganizationForm organization={query.data} />
      )}
    </Page>
  )
}

function OrganizationForm({ organization }: { organization: OrganizationSettings }) {
  const { t, i18n } = useTranslation('organization-settings')
  const { hasPermission } = useSession()
  const queryClient = useQueryClient()
  const canEdit = hasPermission(SETTINGS_MANAGE)
  const fileInput = useRef<HTMLInputElement>(null)
  const [logoFileId, setLogoFileId] = useState<string | null>(organization.logo_file_id ?? null)
  const [logoPreview, setLogoPreview] = useState<string | null>(null)
  const [logoError, setLogoError] = useState<string | null>(null)
  const [uploading, setUploading] = useState(false)

  const {
    register,
    handleSubmit,
    setError,
    setValue,
    reset,
    control,
    formState: { errors, isSubmitting, isDirty },
  } = useForm<FormValues>({ resolver: zodResolver(schema), defaultValues: toFormValues(organization) })

  const primaryColor = useWatch({ control, name: 'primaryColor' })
  const timezone = useWatch({ control, name: 'timezone' })
  const currency = useWatch({ control, name: 'currency' })
  const timezones = useMemo(() => timezoneOptions(timezone), [timezone])
  const currencies = useMemo(() => currencyOptions(currency), [currency])
  const logoChanged = logoFileId !== (organization.logo_file_id ?? null)

  useEffect(
    () => () => {
      if (logoPreview) URL.revokeObjectURL(logoPreview)
    },
    [logoPreview],
  )

  const previewColor = isHexColor(primaryColor) ? primaryColor.toLowerCase() : null
  const palette = previewColor ? brandPalette(previewColor) : null

  const onLogoSelected = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    event.target.value = ''
    if (!file) return
    setLogoError(null)
    if (!LOGO_MIME_TYPES.includes(file.type)) {
      setLogoError(t('logo.invalidType'))
      return
    }
    if (file.size > MAX_LOGO_BYTES) {
      setLogoError(t('logo.tooLarge'))
      return
    }
    setUploading(true)
    try {
      const fileId = await uploadLogo(file)
      setLogoFileId(fileId)
      setLogoPreview(URL.createObjectURL(file))
    } catch {
      setLogoError(t('logo.uploadFailed'))
    } finally {
      setUploading(false)
    }
  }

  const removeLogo = () => {
    setLogoError(null)
    setLogoFileId(null)
    setLogoPreview(null)
  }

  const onSubmit = handleSubmit(async (values) => {
    try {
      const saved = await updateOrganization({
        name: values.name.trim(),
        primary_color: values.primaryColor.trim() === '' ? null : values.primaryColor.trim().toLowerCase(),
        locale: values.locale,
        timezone: values.timezone,
        currency: values.currency,
        logo_file_id: logoFileId,
      })
      queryClient.setQueryData(organizationKeys.settings, saved)
      await queryClient.invalidateQueries({ queryKey: SESSION_QUERY_KEY })
      reset(toFormValues(saved))
      setLogoFileId(saved.logo_file_id ?? null)
      setLogoPreview(null)
      notifySuccess(t('saved'))
    } catch (error) {
      const violations = isApiError(error) ? fieldViolations(error.details) : []
      let handled = false
      for (const violation of violations) {
        if (violation.field === 'logo_file_id') {
          setLogoError(t('logo.invalidType'))
          handled = true
          continue
        }
        const field = serverFields[violation.field]
        if (field) {
          setError(field, { message: serverMessages[field] })
          handled = true
        }
      }
      if (!handled) notifyError(error)
    }
  })

  const errorText = (message?: string) => (message ? t(message as 'errors.nameLength') : undefined)
  const currentLogo = logoPreview ?? (logoFileId ? organizationLogoUrl(logoFileId) : null)
  const busy = isSubmitting || uploading

  return (
    <form className="grid max-w-2xl gap-8" onSubmit={onSubmit} noValidate>
      <UnsavedChangesGuard when={canEdit && (isDirty || logoChanged)} />
      {!canEdit && <p className="rounded-md border bg-muted/50 p-3 text-sm text-muted-foreground">{t('readOnly')}</p>}
      <fieldset className="grid gap-4" disabled={!canEdit || busy}>
        <legend className="mb-2 text-base font-semibold">{t('sections.identity')}</legend>
        <FormRow id="organization-name" label={t('fields.name')} error={errorText(errors.name?.message)}>
          {(describedBy) => (
            <Input placeholder={t('placeholders.name')}
              id="organization-name"
              autoComplete="organization"
              aria-invalid={errors.name ? true : undefined}
              aria-describedby={describedBy}
              {...register('name')}
            />
          )}
        </FormRow>

        <div className="grid gap-2">
          <span className="text-sm font-medium" id="organization-logo-label">
            {t('fields.logo')}
          </span>
          <div className="flex flex-wrap items-center gap-3">
            <div className="flex size-16 items-center justify-center overflow-hidden rounded-md border bg-muted/40">
              {currentLogo ? (
                <img src={currentLogo} alt={t('logo.alt', { name: organization.name })} className="size-full object-contain" />
              ) : (
                <ImageUp className="size-6 text-muted-foreground" aria-hidden />
              )}
            </div>
            <input
              ref={fileInput}
              id="organization-logo"
              type="file"
              accept={LOGO_MIME_TYPES.join(',')}
              className="sr-only"
              aria-labelledby="organization-logo-label"
              aria-describedby="organization-logo-hint"
              onChange={(event) => void onLogoSelected(event)}
            />
            <Button type="button" variant="outline" onClick={() => fileInput.current?.click()}>
              {uploading ? t('logo.uploading') : currentLogo ? t('logo.replace') : t('logo.upload')}
            </Button>
            {currentLogo && (
              <Button type="button" variant="ghost" onClick={removeLogo}>
                <Trash2 aria-hidden />
                {t('logo.remove')}
              </Button>
            )}
          </div>
          <p id="organization-logo-hint" className="text-xs text-muted-foreground">
            {t('logo.hint')}
          </p>
          {logoError && (
            <SharedFieldError>
              {logoError}
            </SharedFieldError>
          )}
        </div>

        <FormRow
          id="organization-primary-color"
          label={t('fields.primaryColor')}
          error={errorText(errors.primaryColor?.message)}
          hint={t('color.hint')}
        >
          {(describedBy) => (
            <div className="flex flex-wrap items-center gap-2">
              <input
                type="color"
                aria-label={t('fields.primaryColorPicker')}
                className="h-9 w-12 cursor-pointer rounded-md border border-input bg-background p-1"
                value={previewColor ?? DEFAULT_PICKER_COLOR}
                onChange={(event) => setValue('primaryColor', event.target.value, { shouldDirty: true, shouldValidate: true })}
              />
              <Input
                id="organization-primary-color"
                className="w-32 font-mono"
                placeholder="#4f46e5"
                aria-invalid={errors.primaryColor ? true : undefined}
                aria-describedby={describedBy}
                {...register('primaryColor')}
              />
              {primaryColor && (
                <Button
                  type="button"
                  variant="ghost"
                  onClick={() => setValue('primaryColor', '', { shouldDirty: true, shouldValidate: true })}
                >
                  {t('color.default')}
                </Button>
              )}
            </div>
          )}
        </FormRow>
        {previewColor && palette && (
          <div className="grid gap-2" aria-live="polite">
            <span className="text-sm font-medium">{t('color.preview')}</span>
            <div className="flex flex-wrap items-center gap-3">
              <span
                className="inline-flex h-9 items-center rounded-md px-4 text-sm font-medium"
                style={{ backgroundColor: palette.solid, color: palette.solidForeground }}
              >
                {t('color.sampleButton')}
              </span>
              <span className="text-sm text-muted-foreground">
                {t('color.contrast', { ratio: formatRatio(contrastRatio(palette.solid, palette.solidForeground)) })}{' '}
                {palette.solid === previewColor ? t('color.contrastOk') : t('color.adjusted')}
              </span>
            </div>
          </div>
        )}
      </fieldset>

      <fieldset className="grid gap-4" disabled={!canEdit || busy}>
        <legend className="mb-2 text-base font-semibold">{t('sections.regional')}</legend>
        <FormRow id="organization-locale" label={t('fields.locale')} error={errorText(errors.locale?.message)}>
          {(describedBy) => (
            <NativeSelect id="organization-locale" aria-describedby={describedBy} {...register('locale')}>
              {LOCALES.map((locale) => (
                <option key={locale} value={locale}>
                  {t(`locales.${locale}`)}
                </option>
              ))}
            </NativeSelect>
          )}
        </FormRow>
        <FormRow
          id="organization-timezone"
          label={t('fields.timezone')}
          error={errorText(errors.timezone?.message)}
          hint={t('hints.timezone')}
        >
          {(describedBy) => (
            <NativeSelect id="organization-timezone" aria-describedby={describedBy} {...register('timezone')}>
              {timezones.map((zone) => (
                <option key={zone} value={zone}>
                  {zone.replaceAll('_', ' ')}
                </option>
              ))}
            </NativeSelect>
          )}
        </FormRow>
        <FormRow
          id="organization-currency"
          label={t('fields.currency')}
          error={errorText(errors.currency?.message)}
          hint={t('hints.currency')}
        >
          {(describedBy) => (
            <NativeSelect id="organization-currency" aria-describedby={describedBy} {...register('currency')}>
              {currencies.map((code) => (
                <option key={code} value={code}>
                  {currencyLabel(code, i18n.language)}
                </option>
              ))}
            </NativeSelect>
          )}
        </FormRow>
      </fieldset>

      {canEdit && (
        <div className="flex justify-end">
          <Button type="submit" disabled={busy || (!isDirty && !logoChanged)}>
            {isSubmitting ? t('saving') : t('save')}
          </Button>
        </div>
      )}
    </form>
  )
}
