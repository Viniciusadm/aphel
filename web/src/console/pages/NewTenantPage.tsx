import { zodResolver } from '@hookform/resolvers/zod'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { Controller, useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { z } from 'zod'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NativeSelect } from '@/components/ui/native-select'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { PasswordInput } from '@/components/ui/password-input'
import { ErrorState } from '@/components/ui/error-state'
import { currencyLabel, currencyOptions, timezoneOptions } from '@/features/settings/organization/lib/preferences'
import { TenantModulePicker } from '../components/TenantModulePicker'
import { consoleApi, consoleKeys } from '../api'
import { FormError } from '../components/FormError'
import { consoleErrorMessage, notifyConsoleSuccess } from '../errors'
import { TENANTS_PATH, tenantPath } from '../paths'
import { emailSchema, hostSchema, passwordSchema, requiredText, slugFrom, slugSchema } from '../schemas'

const LOCALES = ['pt-BR', 'en'] as const

const schema = z.object({
  name: requiredText,
  slug: slugSchema,
  host: hostSchema,
  adminName: requiredText,
  adminEmail: emailSchema,
  adminPassword: passwordSchema,
  blueprint: z.string(),
  modules: z.array(z.string()),
  timezone: requiredText,
  currency: z.string().trim().regex(/^[A-Za-z]{3}$/, 'validation.required'),
  locale: z.enum(LOCALES),
})
const blankSchema = schema.extend({
  name: z.string(),
  slug: z.string(),
  adminName: z.string(),
  adminEmail: z.string(),
  adminPassword: z.string(),
  timezone: z.string(),
  currency: z.string(),
})
type Values = z.infer<typeof schema>

export function NewTenantPage() {
  const { t, i18n } = useTranslation('console')
  const [blank, setBlank] = useState(false)
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [failure, setFailure] = useState<string | null>(null)
  const [slugEdited, setSlugEdited] = useState(false)
  const catalog = useQuery({ queryKey: consoleKeys.catalog, queryFn: consoleApi.catalog })
  const {
    register,
    control,
    handleSubmit,
    setValue,
    clearErrors,
    watch,
    formState: { errors, isSubmitting },
  } = useForm<Values>({
    resolver: zodResolver(blank ? blankSchema : schema),
    defaultValues: {
      name: '',
      slug: '',
      host: '',
      adminName: '',
      adminEmail: '',
      adminPassword: '',
      blueprint: '',
      modules: [],
      timezone: 'America/Sao_Paulo',
      currency: 'BRL',
      locale: 'pt-BR',
    },
  })

  const blueprint = watch('blueprint')
  const profile = catalog.data?.blueprints.find((item) => item.key === blueprint)
  const message = (key: string | undefined) => (key ? t(key as 'validation.required') : undefined)

  const onSubmit = handleSubmit(async (values) => {
    setFailure(null)
    try {
      const tenant = await consoleApi.createTenant(blank ? { mode: 'blank', host: values.host } : {
        mode: 'configured',
        name: values.name,
        slug: values.slug,
        host: values.host,
        admin_name: values.adminName,
        admin_email: values.adminEmail,
        admin_password: values.adminPassword,
        blueprint: values.blueprint || null,
        modules: values.modules,
        timezone: values.timezone,
        currency: values.currency.toUpperCase(),
        locale: values.locale,
      })
      await queryClient.invalidateQueries({ queryKey: consoleKeys.tenants })
      notifyConsoleSuccess(t('create.created'))
      void navigate(tenantPath(tenant.slug))
    } catch (error) {
      setFailure(consoleErrorMessage(error))
      window.scrollTo({ top: 0, behavior: 'smooth' })
    }
  })

  const nameField = register('name', {
    onChange: (event: { target: { value: string } }) => {
      if (!slugEdited) setValue('slug', slugFrom(event.target.value))
    },
  })
  const slugField = register('slug', { onChange: () => setSlugEdited(true) })

  return (
    <Page>
      <PageHeader title={t('create.title')} description={t('create.description')} back={{ to: TENANTS_PATH, label: t('tenant.back') }} />
      <form className="grid gap-6" onSubmit={onSubmit} noValidate>
        <FormError message={failure} />
        <Card><CardHeader><CardTitle>{t('create.startTitle')}</CardTitle><CardDescription>{t('create.startHelp')}</CardDescription></CardHeader>
          <CardContent className="flex flex-wrap gap-3"><Button type="button" aria-pressed={!blank} disabled={isSubmitting} variant={!blank ? 'default' : 'outline'} onClick={() => { setBlank(false); clearErrors() }}>{t('create.customSetup')}</Button>
            <Button type="button" aria-pressed={blank} disabled={isSubmitting} variant={blank ? 'default' : 'outline'} onClick={() => { setBlank(true); setValue('blueprint', ''); setValue('modules', []); clearErrors(); setFailure(null) }}>{t('create.blank')}</Button>
            {blank && <p className="w-full text-sm text-muted-foreground" role="status">{t('create.blankHelp')}</p>}
          </CardContent></Card>
        <Card>
          <CardHeader>
            <CardTitle>{t(blank ? 'create.address' : 'create.company')}</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-4">
            {!blank && <>
              <Field label={t('create.name')} required error={message(errors.name?.message)}>
                {(field) => <Input {...field} autoFocus autoComplete="organization" {...nameField} />}
              </Field>
              <Field label={t('create.slug')} required description={t('create.slugHelp')} error={message(errors.slug?.message)}>
                {(field) => <Input {...field} autoComplete="off" spellCheck={false} {...slugField} />}
              </Field>
            </>}
            <Field label={t('create.host')} required description={t('create.hostHelp')} error={message(errors.host?.message)}>
              {(field) => <Input {...field} autoFocus={blank} inputMode="url" autoComplete="off" spellCheck={false} placeholder={t('create.hostPlaceholder')} {...register('host')} />}
            </Field>
            <div className="rounded-md border border-border bg-muted/40 p-4 text-sm"><p className="font-medium">{t('create.dnsTitle')}</p><p className="mt-2 text-muted-foreground">{t('create.dnsHelp', { ip: catalog.data?.server_public_ip || t('create.serverIpFallback'), recordType: catalog.data?.server_public_ip?.includes(':') ? 'AAAA' : 'A' })}</p><details className="mt-3"><summary className="cursor-pointer font-medium">{t('create.wildcardTitle')}</summary><p className="mt-2 text-muted-foreground">{t('create.wildcardHelp', { ip: catalog.data?.server_public_ip || t('create.serverIpFallback'), recordType: catalog.data?.server_public_ip?.includes(':') ? 'AAAA' : 'A' })}</p></details></div>
          </CardContent>
        </Card>
        {!blank && <>
          <Card>
            <CardHeader>
              <CardTitle>{t('create.admin')}</CardTitle>
            </CardHeader>
            <CardContent className="grid gap-4">
              <Field label={t('create.adminName')} required error={message(errors.adminName?.message)}>
                {(field) => <Input {...field} autoComplete="off" {...register('adminName')} />}
              </Field>
              <Field label={t('create.adminEmail')} required error={message(errors.adminEmail?.message)}>
                {(field) => <Input {...field} type="email" autoComplete="off" {...register('adminEmail')} />}
              </Field>
              <Field
                label={t('create.adminPassword')}
                required
                description={t('create.adminPasswordHelp')}
                error={message(errors.adminPassword?.message)}
              >
                {(field) => <PasswordInput {...field} autoComplete="new-password" {...register('adminPassword')} />}
              </Field>
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>{t('create.setup')}</CardTitle>
              <CardDescription>{t('create.modulesHelp')}</CardDescription>
            </CardHeader>
            <CardContent className="grid gap-4">
              <Field label={t('create.blueprint')}>
                {(field) => (
                  <NativeSelect {...field} {...register('blueprint', { onChange: (event: { target: { value: string } }) => {
                      const chosen = catalog.data?.blueprints.find((item) => item.key === event.target.value)
                      const ready = (key: string, seen = new Set<string>()): boolean => {
                        if (seen.has(key)) return true
                        seen.add(key)
                        const item = catalog.data?.modules.find((candidate) => candidate.key === key)
                        return Boolean(item && !item.requires_configuration && item.dependencies.every((dependency) => ready(dependency, seen)))
                      }
                      setValue('modules', (chosen?.modules ?? []).filter((key) => ready(key)))
                    } })}>
                    <option value="">{t('create.blueprintNone')}</option>
                    {catalog.data?.blueprints.map((blueprint) => (
                      <option key={blueprint.key} value={blueprint.key}>
                        {t(`create.profiles.${blueprint.key}` as 'create.blueprintNone', { defaultValue: t('create.blueprintNone') })}
                      </option>
                    ))}
                  </NativeSelect>
                )}
              </Field>
              <div className="grid gap-4 sm:grid-cols-3">
                <Field label={t('create.timezone')} required error={message(errors.timezone?.message)}>
                  {(field) => <NativeSelect {...field} {...register('timezone')}>{timezoneOptions('America/Sao_Paulo').map((zone) => <option key={zone} value={zone}>{new Intl.DateTimeFormat(i18n.language, { timeZone: zone, timeZoneName: 'long' }).formatToParts(new Date()).find((part) => part.type === 'timeZoneName')?.value} — {zone.split('/').at(-1)?.replaceAll('_', ' ')}</option>)}</NativeSelect>}
                </Field>
                <Field label={t('create.currency')} required error={message(errors.currency?.message)}>
                  {(field) => <NativeSelect {...field} {...register('currency')}>{currencyOptions('BRL').map((code) => <option key={code} value={code}>{currencyLabel(code, i18n.language)}</option>)}</NativeSelect>}
                </Field>
                <Field label={t('create.locale')}>
                  {(field) => (
                    <NativeSelect {...field} {...register('locale')}>
                      {LOCALES.map((locale) => (
                        <option key={locale} value={locale}>
                          {locale === 'pt-BR' ? t('create.portuguese') : t('create.english')}
                        </option>
                      ))}
                    </NativeSelect>
                  )}
                </Field>
              </div>
              {catalog.isPending && <p role="status">{t('create.loadingCatalog')}</p>}
              {catalog.isError && <ErrorState onRetry={() => void catalog.refetch()} />}
              {catalog.data && <Controller control={control} name="modules" render={({ field }) => <TenantModulePicker catalog={catalog.data} selected={field.value} required={profile?.required ?? []} recommended={profile?.modules ?? []} onChange={field.onChange} />} />}
            </CardContent>
          </Card>
        </>}
        <div className="flex flex-wrap justify-end gap-2">
          <Button type="button" variant="outline" onClick={() => void navigate(TENANTS_PATH)}>
            {t('create.cancel')}
          </Button>
          <Button type="submit" disabled={isSubmitting || (!blank && (catalog.isPending || catalog.isError))}>
            {t('create.submit')}
          </Button>
        </div>
      </form>
    </Page>
  )
}
