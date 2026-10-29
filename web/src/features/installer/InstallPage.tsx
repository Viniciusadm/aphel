import { zodResolver } from '@hookform/resolvers/zod'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { LoaderCircle } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { Navigate } from 'react-router'
import { z } from 'zod'
import { api, NETWORK_ERROR_CODE, isApiError } from '@/api'
import { storeSession } from '@/app/session/session-query'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { AuthCard } from '@/features/auth/components/AuthCard'
import { fieldViolations } from '@/features/auth/lib/api-errors'
import { FormRow } from '@/features/settings/organization/components/FormRow'
import { NativeSelect } from '@/features/settings/organization/components/NativeSelect'
import {
  DEFAULT_CURRENCY,
  DEFAULT_LOCALE,
  DEFAULT_TIMEZONE,
  LOCALES,
  browserTimezone,
  currencyLabel,
  currencyOptions,
  timezoneOptions,
} from '@/features/settings/organization/lib/preferences'
import { INSTALLER_STATUS_KEY, install, useInstallerAvailable, type InstallerMe } from './api'
import { EmailStep } from './EmailStep'
import { RestoreStep } from './RestoreStep'
import { preloadInstallerTranslations } from '@/kernel/modules/i18n'
import { ModuleSelection, type InstallerCatalog, type ModuleChoice } from './ModuleSelection'

const schema = z.object({
  organizationName: z.string().trim().min(1, 'errors.required').max(140, 'errors.nameLength'),
  timezone: z.string().min(1, 'errors.timezone'),
  currency: z.string().regex(/^[A-Z]{3}$/, 'errors.currency'),
  locale: z.string().min(1, 'errors.locale'),
  adminName: z.string().trim().min(1, 'errors.required').max(140, 'errors.nameLength'),
  adminEmail: z.string().trim().email('errors.email'),
  adminPassword: z.string().min(8, 'errors.passwordLength').max(128, 'errors.passwordLength'),
})

type FormValues = z.infer<typeof schema>

const serverFields: Record<string, keyof FormValues> = {
  organization_name: 'organizationName',
  timezone: 'timezone',
  currency: 'currency',
  locale: 'locale',
  admin_name: 'adminName',
  admin_email: 'adminEmail',
  admin_password: 'adminPassword',
}

const serverMessages: Record<keyof FormValues, string> = {
  organizationName: 'errors.nameLength',
  timezone: 'errors.timezone',
  currency: 'errors.currency',
  locale: 'errors.locale',
  adminName: 'errors.nameLength',
  adminEmail: 'errors.email',
  adminPassword: 'errors.passwordLength',
}

export function InstallPage() {
  const { t } = useTranslation('installer')
  const { data: available, isPending, isError, refetch } = useInstallerAvailable()
  const [installed, setInstalled] = useState<InstallerMe | null>(null)
  const [restoring, setRestoring] = useState(false)

  if (installed) return <EmailStep me={installed} />

  if (isPending) {
    return (
      <div role="status" className="flex items-center justify-center gap-2 py-12 text-sm text-muted-foreground">
        <LoaderCircle className="size-5 animate-spin" aria-hidden />
        <span>{t('loading')}</span>
      </div>
    )
  }
  if (isError) {
    return (
      <AuthCard title={t('title')} description={t('errors.statusFailed')}>
        <Button type="button" variant="outline" className="w-full" onClick={() => void refetch()}>
          {t('retry')}
        </Button>
      </AuthCard>
    )
  }
  if (!available) return <Navigate to="/login" replace />
  if (restoring) return <RestoreStep onCancel={() => setRestoring(false)} />
  return <InstallForm onInstalled={setInstalled} onRestore={() => setRestoring(true)} />
}

interface InstallFormProps {
  onInstalled: (me: InstallerMe) => void
  onRestore: () => void
}

function InstallForm({ onInstalled, onRestore }: InstallFormProps) {
  const { t, i18n } = useTranslation('installer')
  const queryClient = useQueryClient()
  const [step, setStep] = useState(0)
  const [profile, setProfile] = useState('')
  const [choice, setChoice] = useState<ModuleChoice>({ modules: [], module_settings: {} })
  const catalog = useQuery({ queryKey: [...INSTALLER_STATUS_KEY, 'catalog'], queryFn: async () => { const [result] = await Promise.all([api.get<InstallerCatalog>('/v1/installer'), preloadInstallerTranslations()]); return result }, retry: false })
  const [formError, setFormError] = useState<string | null>(null)
  const timezones = useMemo(() => timezoneOptions(DEFAULT_TIMEZONE), [])
  const currencies = useMemo(() => currencyOptions(DEFAULT_CURRENCY), [])
  const initialTimezone = useMemo(() => {
    const detected = browserTimezone()
    return timezones.includes(detected) ? detected : DEFAULT_TIMEZONE
  }, [timezones])

  const {
    register,
    handleSubmit,
    setError,
    trigger,
    getValues,
    formState: { errors, isSubmitting },
  } = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: {
      organizationName: '',
      timezone: initialTimezone,
      currency: DEFAULT_CURRENCY,
      locale: DEFAULT_LOCALE,
      adminName: '',
      adminEmail: '',
      adminPassword: '',
    },
  })

  const onSubmit = handleSubmit(async (values) => {
    if (step !== 3) return
    setFormError(null)
    try {
      const me = await install({
        ...choice,
        organization_name: values.organizationName.trim(),
        timezone: values.timezone,
        currency: values.currency,
        locale: values.locale,
        admin_name: values.adminName.trim(),
        admin_email: values.adminEmail.trim(),
        admin_password: values.adminPassword,
        public_base_url: window.location.origin,
      })
      storeSession(queryClient, me)
      setChoice({ modules: [], module_settings: {} })
      onInstalled(me)
      queryClient.setQueryData(INSTALLER_STATUS_KEY, false)
    } catch (error) {
      if (!isApiError(error)) {
        setFormError(t('errors.generic'))
        return
      }
      if (error.status === 404) {
        queryClient.setQueryData(INSTALLER_STATUS_KEY, false)
        return
      }
      if (error.code === 'email_taken') {
        setError('adminEmail', { message: 'errors.emailTaken' })
        return
      }
      const violations = fieldViolations(error.details)
      let handled = false
      for (const violation of violations) {
        const field = serverFields[violation.field]
        if (!field) continue
        setError(field, { message: serverMessages[field] })
        handled = true
      }
      if (violations.some((violation) => violation.field === 'modules' || violation.field.startsWith('module_settings.'))) {
        setStep(2)
        setFormError(t('selection.invalidConfiguration'))
        return
      }
      if (handled) setStep(0)
      if (!handled) setFormError(error.code === NETWORK_ERROR_CODE ? t('errors.network') : t('errors.generic'))
    }
  }, () => setStep(0))

  const message = (key?: string) => (key ? t(key as 'errors.required') : undefined)

  return (
    <AuthCard title={t('title')} description={t('description')}>
      <p className="mb-4 text-sm text-muted-foreground" role="status">{t('selection.step', { current: step + 1, total: 4 })}</p>
      <form className="grid gap-6" onSubmit={onSubmit} noValidate>
        {formError && (
          <p role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">
            {formError}
          </p>
        )}
        <fieldset hidden={step !== 0} className={step === 0 ? "grid gap-4" : "hidden"} disabled={isSubmitting}>
          <legend className="mb-2 text-sm font-semibold">{t('sections.organization')}</legend>
          <FormRow id="install-organization-name" label={t('fields.organizationName')} error={message(errors.organizationName?.message)}>
            {(describedBy) => (
              <Input
                id="install-organization-name"
                autoComplete="organization"
                aria-invalid={errors.organizationName ? true : undefined}
                aria-describedby={describedBy}
                {...register('organizationName')}
              />
            )}
          </FormRow>
          <FormRow
            id="install-timezone"
            label={t('fields.timezone')}
            error={message(errors.timezone?.message)}
            hint={t('hints.timezone')}
          >
            {(describedBy) => (
              <NativeSelect id="install-timezone" aria-describedby={describedBy} {...register('timezone')}>
                {timezones.map((zone) => (
                  <option key={zone} value={zone}>
                    {zone.replaceAll('_', ' ')}
                  </option>
                ))}
              </NativeSelect>
            )}
          </FormRow>
          <FormRow id="install-currency" label={t('fields.currency')} error={message(errors.currency?.message)}>
            {(describedBy) => (
              <NativeSelect id="install-currency" aria-describedby={describedBy} {...register('currency')}>
                {currencies.map((code) => (
                  <option key={code} value={code}>
                    {currencyLabel(code, i18n.language)}
                  </option>
                ))}
              </NativeSelect>
            )}
          </FormRow>
          <FormRow id="install-locale" label={t('fields.locale')} error={message(errors.locale?.message)}>
            {(describedBy) => (
              <NativeSelect id="install-locale" aria-describedby={describedBy} {...register('locale')}>
                {LOCALES.map((locale) => (
                  <option key={locale} value={locale}>
                    {t(`locales.${locale}`)}
                  </option>
                ))}
              </NativeSelect>
            )}
          </FormRow>
        </fieldset>
        <fieldset hidden={step !== 0} className={step === 0 ? "grid gap-4" : "hidden"} disabled={isSubmitting}>
          <legend className="mb-2 text-sm font-semibold">{t('sections.admin')}</legend>
          <FormRow id="install-admin-name" label={t('fields.adminName')} error={message(errors.adminName?.message)}>
            {(describedBy) => (
              <Input
                id="install-admin-name"
                autoComplete="name"
                aria-invalid={errors.adminName ? true : undefined}
                aria-describedby={describedBy}
                {...register('adminName')}
              />
            )}
          </FormRow>
          <FormRow id="install-admin-email" label={t('fields.adminEmail')} error={message(errors.adminEmail?.message)}>
            {(describedBy) => (
              <Input
                id="install-admin-email"
                type="email"
                autoComplete="email"
                aria-invalid={errors.adminEmail ? true : undefined}
                aria-describedby={describedBy}
                {...register('adminEmail')}
              />
            )}
          </FormRow>
          <FormRow
            id="install-admin-password"
            label={t('fields.adminPassword')}
            error={message(errors.adminPassword?.message)}
            hint={errors.adminPassword ? undefined : t('hints.password')}
          >
            {(describedBy) => (
              <Input
                id="install-admin-password"
                type="password"
                autoComplete="new-password"
                aria-invalid={errors.adminPassword ? true : undefined}
                aria-describedby={describedBy}
                {...register('adminPassword')}
              />
            )}
          </FormRow>
        </fieldset>
        {step === 3 && <dl className="grid gap-2 rounded-lg border p-4 text-sm"><div><dt className="text-muted-foreground">{t('fields.organizationName')}</dt><dd>{getValues('organizationName')}</dd></div><div><dt className="text-muted-foreground">{t('fields.adminEmail')}</dt><dd>{getValues('adminEmail')}</dd></div><div><dt className="text-muted-foreground">{t('fields.timezone')}</dt><dd>{getValues('timezone')}</dd></div><div><dt className="text-muted-foreground">{t('fields.currency')}</dt><dd>{getValues('currency')}</dd></div></dl>}
        {step > 0 && (catalog.isPending ? <p role="status">{t('loading')}</p> : catalog.data ? <fieldset disabled={isSubmitting} className="min-w-0"><ModuleSelection catalog={catalog.data} value={choice} onChange={setChoice} profile={profile} onProfileChange={setProfile} stage={step === 1 ? 'profiles' : step === 2 ? 'shelf' : 'review'} /></fieldset> : <div role="alert"><p>{t('errors.statusFailed')}</p><Button type="button" variant="outline" onClick={() => void catalog.refetch()}>{t('retry')}</Button></div>)}
        <div className="flex gap-2">
          {step > 0 && <Button type="button" variant="outline" disabled={isSubmitting} onClick={() => { setFormError(null); setStep(step - 1) }}>{t('selection.back')}</Button>}
          {step < 3 ? <Button type="button" className="flex-1" disabled={isSubmitting || (step > 0 && !catalog.data)} onClick={() => { void (async () => { if (step === 0 && !await trigger()) return; setFormError(null); setStep(step + 1) })() }}>{t('selection.next')}</Button> : <Button type="submit" className="flex-1" disabled={isSubmitting}>{isSubmitting ? t('submitting') : t('submit')}</Button>}
        </div>
        {step === 0 && <Button type="button" variant="ghost" className="w-full" disabled={isSubmitting} onClick={onRestore}>{t('restore.open')}</Button>}
      </form>
    </AuthCard>
  )
}
