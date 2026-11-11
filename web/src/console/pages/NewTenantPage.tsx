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
import { ToggleField } from '@/components/ui/toggle-field'
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
type Values = z.infer<typeof schema>

export function NewTenantPage() {
  const { t } = useTranslation('console')
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
    formState: { errors, isSubmitting },
  } = useForm<Values>({
    resolver: zodResolver(schema),
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

  const message = (key: string | undefined) => (key ? t(key as 'validation.required') : undefined)

  const onSubmit = handleSubmit(async (values) => {
    setFailure(null)
    try {
      const tenant = await consoleApi.createTenant({
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
    <Page width="form">
      <PageHeader title={t('create.title')} description={t('create.description')} back={{ to: TENANTS_PATH, label: t('tenant.back') }} />
      <form className="grid gap-6" onSubmit={onSubmit} noValidate>
        <FormError message={failure} />
        <Card>
          <CardHeader>
            <CardTitle>{t('create.company')}</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-4">
            <Field label={t('create.name')} required error={message(errors.name?.message)}>
              {(field) => <Input {...field} autoFocus autoComplete="organization" {...nameField} />}
            </Field>
            <Field label={t('create.slug')} required description={t('create.slugHelp')} error={message(errors.slug?.message)}>
              {(field) => <Input {...field} autoComplete="off" spellCheck={false} {...slugField} />}
            </Field>
            <Field label={t('create.host')} required description={t('create.hostHelp')} error={message(errors.host?.message)}>
              {(field) => <Input {...field} inputMode="url" autoComplete="off" spellCheck={false} placeholder="empresa.exemplo.com" {...register('host')} />}
            </Field>
          </CardContent>
        </Card>
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
              {(field) => <Input {...field} type="password" autoComplete="new-password" {...register('adminPassword')} />}
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
                <NativeSelect {...field} {...register('blueprint')}>
                  <option value="">{t('create.blueprintNone')}</option>
                  {catalog.data?.blueprints.map((blueprint) => (
                    <option key={blueprint.key} value={blueprint.key}>
                      {blueprint.key}
                    </option>
                  ))}
                </NativeSelect>
              )}
            </Field>
            <div className="grid gap-4 sm:grid-cols-3">
              <Field label={t('create.timezone')} required error={message(errors.timezone?.message)}>
                {(field) => <Input {...field} autoComplete="off" {...register('timezone')} />}
              </Field>
              <Field label={t('create.currency')} required error={message(errors.currency?.message)}>
                {(field) => <Input {...field} maxLength={3} autoComplete="off" className="uppercase" {...register('currency')} />}
              </Field>
              <Field label={t('create.locale')}>
                {(field) => (
                  <NativeSelect {...field} {...register('locale')}>
                    {LOCALES.map((locale) => (
                      <option key={locale} value={locale}>
                        {locale}
                      </option>
                    ))}
                  </NativeSelect>
                )}
              </Field>
            </div>
            {catalog.data && catalog.data.modules.length > 0 && (
              <fieldset className="grid gap-2">
                <legend className="mb-2 text-sm font-medium">{t('create.modules')}</legend>
                <Controller
                  control={control}
                  name="modules"
                  render={({ field }) => (
                    <div className="grid gap-3 sm:grid-cols-2">
                      {catalog.data.modules.map((module) => (
                        <ToggleField
                          key={module.key}
                          label={module.name}
                          description={module.description}
                          checked={field.value.includes(module.key)}
                          onCheckedChange={(checked) =>
                            field.onChange(checked ? [...field.value, module.key] : field.value.filter((key) => key !== module.key))
                          }
                        />
                      ))}
                    </div>
                  )}
                />
              </fieldset>
            )}
          </CardContent>
        </Card>
        <div className="flex flex-wrap justify-end gap-2">
          <Button type="button" variant="outline" onClick={() => void navigate(TENANTS_PATH)}>
            {t('create.cancel')}
          </Button>
          <Button type="submit" disabled={isSubmitting}>
            {t('create.submit')}
          </Button>
        </div>
      </form>
    </Page>
  )
}
