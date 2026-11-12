import { zodResolver } from '@hookform/resolvers/zod'
import { ArrowRight, ExternalLink, Plus, RefreshCw, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { Controller, useForm, useWatch } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { z } from 'zod'
import { useSession } from '@/app/session/session-context'
import { notifyError, notifySuccess, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { ToggleField } from '@/components/ui/toggle-field'
import {
  FormRow,
  type OrganizationConfiguration,
  serverFieldErrors,
  SettingsNativeSelect as NativeSelect,
  SETTINGS_MANAGE,
  SYSTEM_MANAGE,
  useAllMembers,
} from '@/sdk'
import {
  checkDomains,
  siteApi,
  type DomainCheck,
  useInvalidateSite,
  useSiteCustomers,
  useSitePathRedirects,
  useSiteRedirects,
  type SiteOverview,
} from './api'
import { currentSettings, languageName, type SiteModuleSettings, useSaveSite } from './siteSettings'

const RESERVED_SEGMENTS = ['api']

function segmentOf(basePath: string): string {
  return basePath.replace(/^\/+|\/+$/g, '')
}

function basePathOf(segment: string): string {
  const trimmed = segment.trim().toLowerCase()
  return trimmed ? `/${trimmed}` : '/'
}

export function SiteSummary({ overview }: { overview: SiteOverview }) {
  const { t } = useTranslation('site')
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-card px-4 py-3">
      <div className="grid min-w-0 gap-0.5">
        <span className="truncate text-sm font-medium">{overview.base_url}</span>
        <span className="text-sm text-muted-foreground">
          {overview.pages > 0 ? t('page.pages', { count: overview.pages }) : t('page.noPages')}
        </span>
      </div>
      <Button asChild variant="outline" size="sm">
        <a href={`${overview.base_url}/`} target="_blank" rel="noreferrer">
          {t('page.open')}
          <ExternalLink aria-hidden="true" />
        </a>
      </Button>
    </div>
  )
}

const HOST_PATTERN = /^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)*(:\d{1,5})?$/

function hostList(value: string): string[] {
  return value
    .split(/[\s,]+/)
    .map((host) => host.trim().toLowerCase())
    .filter(Boolean)
}

const addressSchema = (appSegment: string) =>
  z
    .object({
      host: z
        .string()
        .trim()
        .toLowerCase()
        .regex(/^([a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)*(:\d{1,5})?)?$/, 'errors.host'),
      segment: z
        .string()
        .trim()
        .toLowerCase()
        .regex(/^([a-z0-9]([a-z0-9-]{0,38}[a-z0-9])?)?$/, 'errors.path')
        .refine((value) => !RESERVED_SEGMENTS.includes(value), 'errors.pathReserved'),
      redirectHosts: z
        .string()
        .refine((value) => hostList(value).every((host) => HOST_PATTERN.test(host)), 'errors.host')
        .refine((value) => hostList(value).length <= 10, 'errors.redirectHostsMax'),
      indexing: z.boolean(),
    })
    .refine((values) => values.host.trim() !== '' || values.segment.trim().toLowerCase() !== appSegment, {
      path: ['segment'],
      message: 'errors.pathTaken',
    })

type AddressValues = z.infer<ReturnType<typeof addressSchema>>

export function AddressCard({ configuration }: { configuration: OrganizationConfiguration }) {
  const { t } = useTranslation('site')
  const { hasPermission } = useSession()
  const canMove = hasPermission(SYSTEM_MANAGE)
  const saveSite = useSaveSite(configuration)
  const invalidate = useInvalidateSite()
  const appSegment = segmentOf(configuration.app_base_path)
  const defaults: AddressValues = {
    host: configuration.site_host ?? '',
    redirectHosts: (configuration.site_redirect_hosts ?? []).join(', '),
    segment: segmentOf(configuration.site_base_path),
    indexing: currentSettings(configuration).indexing,
  }
  const {
    register,
    control,
    handleSubmit,
    reset,
    setError,
    formState: { errors, isSubmitting, isDirty },
  } = useForm<AddressValues>({ resolver: zodResolver(addressSchema(appSegment)), defaultValues: defaults })
  const host = useWatch({ control, name: 'host' })
  const redirectHosts = useWatch({ control, name: 'redirectHosts' })
  const publicBase = configuration.effective_public_base_url.replace(/\/+$/, '')
  const prefix = host.trim()
    ? `${publicBase.startsWith('https://') ? 'https' : 'http'}://${host.trim().toLowerCase()}/`
    : `${publicBase}/`
  const errorText = (message?: string) => (message?.startsWith('errors.') ? t(message as 'errors.path') : message)

  const [checks, setChecks] = useState<DomainCheck[] | null>(null)
  const [checking, setChecking] = useState(false)
  const verifyDomains = async () => {
    setChecking(true)
    try {
      setChecks(await checkDomains())
    } catch (error) {
      notifyError(error)
    } finally {
      setChecking(false)
    }
  }

  const onSubmit = handleSubmit(async (values) => {
    try {
      const saved = await saveSite(
        { indexing: values.indexing },
        canMove
          ? {
              site_base_path: basePathOf(values.segment),
              site_host: values.host.trim().toLowerCase() || null,
              site_redirect_hosts: values.host.trim() ? hostList(values.redirectHosts) : [],
            }
          : {},
      )
      reset({
        host: saved.site_host ?? '',
        redirectHosts: (saved.site_redirect_hosts ?? []).join(', '),
        segment: segmentOf(saved.site_base_path),
        indexing: saved.site?.indexing ?? true,
      })
      void invalidate()
      notifySuccess(t('saved'))
    } catch (error) {
      const fieldErrors = serverFieldErrors(error, {
        'mounts.site.base_path': 'segment',
        'mounts.site.host': 'host',
        'mounts.site.redirect_hosts': 'redirectHosts',
      })
      for (const [field, message] of fieldErrors) setError(field as keyof AddressValues, { message })
      if (fieldErrors.length === 0) notifyError(error)
    }
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('address.title')}</CardTitle>
        <CardDescription>{t('address.description')}</CardDescription>
      </CardHeader>
      <CardContent>
        <form className="grid gap-4" onSubmit={onSubmit} noValidate>
          <UnsavedChangesGuard when={isDirty} />
          <fieldset className="grid gap-4" disabled={isSubmitting}>
            <FormRow
              id="site-host"
              label={t('address.host')}
              error={errorText(errors.host?.message)}
              hint={canMove ? t('address.hostHint') : t('address.locked')}
            >
              {(describedBy) => (
                <Input
                  id="site-host"
                  placeholder={t('address.hostPlaceholder')}
                  autoComplete="off"
                  spellCheck={false}
                  inputMode="url"
                  readOnly={!canMove}
                  aria-invalid={errors.host ? true : undefined}
                  aria-describedby={describedBy}
                  {...register('host')}
                />
              )}
            </FormRow>
            {host.trim() && (
              <FormRow
                id="site-redirect-hosts"
                label={t('address.redirectHosts')}
                error={errorText(errors.redirectHosts?.message)}
                hint={canMove ? t('address.redirectHostsHint', { host: host.trim().toLowerCase() }) : t('address.locked')}
              >
                {(describedBy) => (
                  <Input
                    id="site-redirect-hosts"
                    placeholder={t('address.redirectHostsPlaceholder')}
                    autoComplete="off"
                    spellCheck={false}
                    readOnly={!canMove}
                    aria-invalid={errors.redirectHosts ? true : undefined}
                    aria-describedby={describedBy}
                    {...register('redirectHosts')}
                  />
                )}
              </FormRow>
            )}
            {host.trim() && !errors.host && (
              <div className="grid gap-1 rounded-md border bg-muted/40 p-3 text-sm">
                <span className="font-medium">{t('address.dnsTitle')}</span>
                <span className="text-muted-foreground">
                  {t('address.dnsPoint', { hosts: [host.trim().toLowerCase(), ...hostList(redirectHosts)].join(', ') })}
                </span>
                <span className="text-muted-foreground">{t('address.dnsTls')}</span>
                {canMove && !isDirty && (
                  <div className="grid gap-1 pt-1">
                    <Button type="button" variant="outline" size="sm" className="justify-self-start" disabled={checking} onClick={() => void verifyDomains()}>
                      {t('address.dnsCheck')}
                    </Button>
                    {checks?.map((check) => (
                      <span key={check.host} role="status" className={check.resolves ? 'text-success' : 'text-destructive'}>
                        {check.resolves ? t('address.dnsResolves', { host: check.host, addresses: check.addresses.join(', ') }) : t('address.dnsMissing', { host: check.host })}
                      </span>
                    ))}
                  </div>
                )}
              </div>
            )}
            <FormRow
              id="site-path"
              label={t('address.path')}
              error={errorText(errors.segment?.message)}
              hint={canMove ? t('address.pathHint') : t('address.locked')}
            >
              {(describedBy) => (
                <div className="flex min-w-0 items-center rounded-md border border-input bg-muted/40 shadow-xs">
                  <span className="truncate pl-3 text-sm text-muted-foreground">{prefix}</span>
                  <Input
                    id="site-path"
                    className="min-w-24 flex-1 border-0 shadow-none"
                    placeholder={t('address.pathPlaceholder')}
                    autoComplete="off"
                    spellCheck={false}
                    readOnly={!canMove}
                    aria-invalid={errors.segment ? true : undefined}
                    aria-describedby={describedBy}
                    {...register('segment')}
                  />
                </div>
              )}
            </FormRow>
            <Controller
              control={control}
              name="indexing"
              render={({ field }) => (
                <ToggleField
                  id="site-indexing"
                  type="switch"
                  label={t('address.indexing')}
                  description={t('address.indexingHint')}
                  checked={field.value}
                  onCheckedChange={(checked) => field.onChange(checked === true)}
                />
              )}
            />
          </fieldset>
          <div className="flex justify-end">
            <Button type="submit" disabled={isSubmitting || !isDirty}>
              {t('address.save')}
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  )
}

const appearanceSchema = z.object({
  scrollbar: z.enum(['default', 'themed']),
})

type AppearanceValues = z.infer<typeof appearanceSchema>

export function AppearanceCard({ configuration }: { configuration: OrganizationConfiguration }) {
  const { t } = useTranslation('site')
  const saveSite = useSaveSite(configuration)
  const invalidate = useInvalidateSite()
  const settings = currentSettings(configuration)
  const {
    register,
    handleSubmit,
    reset,
    formState: { isSubmitting, isDirty },
  } = useForm<AppearanceValues>({
    resolver: zodResolver(appearanceSchema),
    defaultValues: { scrollbar: settings.scrollbar ?? 'default' },
  })
  const onSubmit = handleSubmit(async (values) => {
    try {
      const saved = await saveSite({ scrollbar: values.scrollbar })
      reset({ scrollbar: saved.site?.scrollbar ?? 'default' })
      void invalidate()
      notifySuccess(t('saved'))
    } catch (error) {
      notifyError(error)
    }
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('appearance.title')}</CardTitle>
        <CardDescription>{t('appearance.description')}</CardDescription>
      </CardHeader>
      <CardContent>
        <form className="grid gap-4" onSubmit={onSubmit} noValidate>
          <UnsavedChangesGuard when={isDirty} />
          <fieldset className="grid gap-4" disabled={isSubmitting}>
            <FormRow id="site-scrollbar" label={t('appearance.scrollbar')} hint={t('appearance.scrollbarHint')}>
              {(describedBy) => (
                <NativeSelect id="site-scrollbar" className="min-w-48" aria-describedby={describedBy} {...register('scrollbar')}>
                  <option value="default">{t('appearance.scrollbarDefault')}</option>
                  <option value="themed">{t('appearance.scrollbarThemed')}</option>
                </NativeSelect>
              )}
            </FormRow>
          </fieldset>
          <div className="flex justify-end">
            <Button type="submit" disabled={isSubmitting || !isDirty}>
              {t('appearance.save')}
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  )
}

export function LanguagesCard({ configuration, overview }: { configuration: OrganizationConfiguration; overview: SiteOverview }) {
  const { t, i18n } = useTranslation('site')
  const saveSite = useSaveSite(configuration)
  const invalidate = useInvalidateSite()
  const [saving, setSaving] = useState(false)
  const selected = currentSettings(configuration).languages

  const toggle = async (code: string, checked: boolean) => {
    const next = checked ? [...selected, code] : selected.filter((candidate) => candidate !== code)
    setSaving(true)
    try {
      await saveSite({ languages: overview.available_languages.filter((candidate) => next.includes(candidate)) })
      void invalidate()
      notifySuccess(t('saved'))
    } catch (error) {
      notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('languages.title')}</CardTitle>
        <CardDescription>
          {t('languages.description', { language: languageName(overview.default_language, i18n.language) })}
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-3">
        <fieldset className="grid gap-2" disabled={saving}>
          <legend className="mb-1 text-sm font-medium">{t('languages.extra')}</legend>
          {overview.available_languages.map((code) => (
            <ToggleField
              key={code}
              id={`site-language-${code}`}
              label={languageName(code, i18n.language)}
              description={`${overview.base_url}/${code.toLowerCase()}/`}
              checked={selected.includes(code)}
              onCheckedChange={(checked) => void toggle(code, checked === true)}
            />
          ))}
        </fieldset>
        <p className="text-sm text-muted-foreground">{t('languages.hint')}</p>
      </CardContent>
    </Card>
  )
}

export function CustomersCard({ configuration, overview }: { configuration: OrganizationConfiguration; overview: SiteOverview }) {
  const { t, i18n } = useTranslation('site')
  const saveSite = useSaveSite(configuration)
  const invalidate = useInvalidateSite()
  const [search, setSearch] = useState('')
  const [saving, setSaving] = useState(false)
  const settings = currentSettings(configuration)
  const enabled = settings.accounts || overview.customers_required
  const customers = useSiteCustomers(search.trim())
  const members = useAllMembers()
  const activeMembers = (members.data ?? []).filter((member) => member.status === 'active')
  const formatDate = (value: string | null) =>
    value ? new Intl.DateTimeFormat(i18n.language, { dateStyle: 'short' }).format(new Date(value)) : '—'

  const save = async (site: Partial<SiteModuleSettings>) => {
    setSaving(true)
    try {
      await saveSite(site)
      void invalidate()
      notifySuccess(t('saved'))
    } catch (error) {
      notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('customers.title')}</CardTitle>
        <CardDescription>{t('customers.description')}</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-4">
        <ToggleField
          id="site-accounts"
          type="switch"
          label={t('customers.enable')}
          description={overview.customers_required ? t('customers.required') : t('customers.enableHint')}
          checked={enabled}
          disabled={saving || overview.customers_required}
          onCheckedChange={(checked) => void save({ accounts: checked === true })}
        />
        {enabled && overview.people_available && (
          <div className="grid gap-3 rounded-md border bg-muted/30 p-3">
            <ToggleField
              id="site-link-people"
              type="switch"
              label={t('customers.linkPeople')}
              description={t('customers.linkPeopleHint')}
              checked={settings.link_people}
              disabled={saving}
              onCheckedChange={(checked) => void save({ link_people: checked === true })}
            />
            {settings.link_people && (
              <FormRow id="site-people-owner" label={t('customers.owner')} hint={t('customers.ownerHint')}>
                {(describedBy) => (
                  <NativeSelect
                    id="site-people-owner"
                    aria-describedby={describedBy}
                    value={settings.people_owner ?? ''}
                    disabled={saving || members.isPending}
                    onChange={(event) => void save({ people_owner: event.target.value || null })}
                  >
                    <option value="">{t('customers.noOwner')}</option>
                    {activeMembers.map((member) => (
                      <option key={member.user_id} value={member.user_id}>
                        {member.display_name}
                      </option>
                    ))}
                  </NativeSelect>
                )}
              </FormRow>
            )}
          </div>
        )}
        {enabled && (
          <>
            <FormRow id="site-customer-search" label={t('customers.search')}>
              {(describedBy) => (
                <Input
                  id="site-customer-search"
                  type="search"
                  value={search}
                  placeholder={t('customers.searchPlaceholder')}
                  aria-describedby={describedBy}
                  onChange={(event) => setSearch(event.target.value)}
                />
              )}
            </FormRow>
            {customers.isPending ? (
              <Skeleton className="h-16" />
            ) : customers.isError ? (
              <ErrorState onRetry={() => void customers.refetch()} />
            ) : customers.data.data.length === 0 ? (
              <EmptyState title={search ? t('customers.noMatch') : t('customers.empty')} />
            ) : (
              <>
                <p className="text-sm text-muted-foreground">{t('customers.total', { count: customers.data.meta.total })}</p>
                <ul className="grid gap-2" aria-label={t('customers.listLabel')}>
                  {customers.data.data.map((customer) => (
                    <li key={customer.id} className="flex flex-wrap items-center justify-between gap-2 rounded-md border bg-card px-3 py-2">
                      <div className="grid min-w-0 gap-0.5">
                        <span className="truncate text-sm font-medium">{customer.name || customer.email}</span>
                        <span className="truncate text-xs text-muted-foreground">
                          {customer.email}
                          {customer.email_verified_at ? '' : ` · ${t('customers.unverified')}`}
                        </span>
                      </div>
                      <span className="text-xs text-muted-foreground">
                        {t('customers.since', { date: formatDate(customer.created_at) })}
                      </span>
                    </li>
                  ))}
                </ul>
              </>
            )}
          </>
        )}
      </CardContent>
    </Card>
  )
}

const pathRedirectSchema = z.object({
  source_path: z
    .string()
    .trim()
    .regex(/^\/[^\s?#]*$/, 'errors.redirectSource'),
  target: z
    .string()
    .trim()
    .regex(/^(\/(?!\/)\S*|https?:\/\/\S+)$/, 'errors.redirectTarget'),
  permanent: z.boolean(),
})

type PathRedirectValues = z.infer<typeof pathRedirectSchema>

const EMPTY_PATH_REDIRECT: PathRedirectValues = { source_path: '', target: '', permanent: true }

export function PathRedirectsCard() {
  const { t } = useTranslation('site')
  const redirects = useSitePathRedirects()
  const invalidate = useInvalidateSite()
  const [removing, setRemoving] = useState<number | null>(null)
  const {
    register,
    control,
    handleSubmit,
    reset,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<PathRedirectValues>({ resolver: zodResolver(pathRedirectSchema), defaultValues: EMPTY_PATH_REDIRECT })
  const errorText = (message?: string) => (message?.startsWith('errors.') ? t(message as 'errors.redirectSource') : message)

  const onSubmit = handleSubmit(async (values) => {
    try {
      await siteApi.createPathRedirect(values)
      reset(EMPTY_PATH_REDIRECT)
      await invalidate()
      notifySuccess(t('pathRedirects.added'))
    } catch (error) {
      const fieldErrors = serverFieldErrors(error, {})
      for (const [field, message] of fieldErrors) setError(field as keyof PathRedirectValues, { message })
      if (fieldErrors.length === 0) notifyError(error)
    }
  })

  const remove = async (id: number) => {
    setRemoving(id)
    try {
      await siteApi.deletePathRedirect(id)
      await invalidate()
      notifySuccess(t('redirects.removed'))
    } catch (error) {
      notifyError(error)
    } finally {
      setRemoving(null)
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('pathRedirects.title')}</CardTitle>
        <CardDescription>{t('pathRedirects.description')}</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-4">
        <form className="grid gap-4" onSubmit={onSubmit} noValidate>
          <fieldset className="grid gap-4 sm:grid-cols-2" disabled={isSubmitting}>
            <FormRow
              id="site-redirect-source"
              label={t('pathRedirects.source')}
              error={errorText(errors.source_path?.message)}
              hint={t('pathRedirects.sourceHint')}
            >
              {(describedBy) => (
                <Input
                  id="site-redirect-source"
                  placeholder="/pagina-antiga"
                  autoComplete="off"
                  spellCheck={false}
                  aria-invalid={errors.source_path ? true : undefined}
                  aria-describedby={describedBy}
                  {...register('source_path')}
                />
              )}
            </FormRow>
            <FormRow
              id="site-redirect-target"
              label={t('pathRedirects.target')}
              error={errorText(errors.target?.message)}
              hint={t('pathRedirects.targetHint')}
            >
              {(describedBy) => (
                <Input
                  id="site-redirect-target"
                  placeholder="/blog"
                  autoComplete="off"
                  spellCheck={false}
                  inputMode="url"
                  aria-invalid={errors.target ? true : undefined}
                  aria-describedby={describedBy}
                  {...register('target')}
                />
              )}
            </FormRow>
            <div className="sm:col-span-2">
              <Controller
                control={control}
                name="permanent"
                render={({ field }) => (
                  <ToggleField
                    id="site-redirect-permanent"
                    type="switch"
                    label={t('pathRedirects.permanent')}
                    description={t('pathRedirects.permanentHint')}
                    checked={field.value}
                    onCheckedChange={(checked) => field.onChange(checked === true)}
                  />
                )}
              />
            </div>
          </fieldset>
          <div className="flex justify-end">
            <Button type="submit" disabled={isSubmitting}>
              <Plus aria-hidden="true" />
              {t('pathRedirects.add')}
            </Button>
          </div>
        </form>
        {redirects.isPending ? (
          <Skeleton className="h-16" />
        ) : redirects.isError ? (
          <ErrorState onRetry={() => void redirects.refetch()} />
        ) : redirects.data.length === 0 ? (
          <EmptyState title={t('pathRedirects.empty')} />
        ) : (
          <ul className="grid gap-2" aria-label={t('pathRedirects.listLabel')}>
            {redirects.data.map((redirect) => (
              <li key={redirect.id} className="flex flex-wrap items-center gap-2 rounded-md border bg-card px-3 py-2">
                <code className="min-w-0 truncate text-sm">{redirect.source_path}</code>
                <ArrowRight className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
                <code className="min-w-0 flex-1 truncate text-sm">{redirect.target}</code>
                {!redirect.permanent && <span className="text-xs text-muted-foreground">{t('pathRedirects.temporary')}</span>}
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={removing === redirect.id}
                  onClick={() => void remove(redirect.id)}
                  aria-label={t('redirects.remove', { path: redirect.source_path })}
                >
                  <Trash2 aria-hidden="true" />
                </Button>
              </li>
            ))}
          </ul>
        )}
      </CardContent>
    </Card>
  )
}

export function RedirectsCard() {
  const { t } = useTranslation('site')
  const redirects = useSiteRedirects()
  const invalidate = useInvalidateSite()
  const [removing, setRemoving] = useState<number | null>(null)

  const remove = async (id: number) => {
    setRemoving(id)
    try {
      await siteApi.deleteRedirect(id)
      await invalidate()
      notifySuccess(t('redirects.removed'))
    } catch (error) {
      notifyError(error)
    } finally {
      setRemoving(null)
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('redirects.title')}</CardTitle>
        <CardDescription>{t('redirects.description')}</CardDescription>
      </CardHeader>
      <CardContent>
        {redirects.isPending ? (
          <Skeleton className="h-16" />
        ) : redirects.isError ? (
          <ErrorState onRetry={() => void redirects.refetch()} />
        ) : redirects.data.length === 0 ? (
          <EmptyState title={t('redirects.empty')} />
        ) : (
          <ul className="grid gap-2" aria-label={t('redirects.listLabel')}>
            {redirects.data.map((redirect) => {
              const from = redirect.old_path ?? redirect.old_value
              const to = redirect.new_path ?? redirect.new_value
              return (
                <li key={redirect.id} className="flex flex-wrap items-center gap-2 rounded-md border bg-card px-3 py-2">
                  <code className="min-w-0 truncate text-sm">{from}</code>
                  <ArrowRight className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
                  <code className="min-w-0 flex-1 truncate text-sm">{to}</code>
                  <Button
                    size="sm"
                    variant="ghost"
                    disabled={removing === redirect.id}
                    onClick={() => void remove(redirect.id)}
                    aria-label={t('redirects.remove', { path: from })}
                  >
                    <Trash2 aria-hidden="true" />
                  </Button>
                </li>
              )
            })}
          </ul>
        )}
      </CardContent>
    </Card>
  )
}

export function CacheCard() {
  const { t } = useTranslation('site')
  const { hasPermission } = useSession()
  const [busy, setBusy] = useState(false)

  const purge = async () => {
    setBusy(true)
    try {
      await siteApi.purgeCache()
      notifySuccess(t('cache.purged'))
    } catch (error) {
      notifyError(error)
    } finally {
      setBusy(false)
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('cache.title')}</CardTitle>
        <CardDescription>{t('cache.description')}</CardDescription>
      </CardHeader>
      <CardContent>
        <Button variant="outline" disabled={busy || !hasPermission(SETTINGS_MANAGE)} onClick={() => void purge()}>
          <RefreshCw aria-hidden="true" />
          {t('cache.purge')}
        </Button>
      </CardContent>
    </Card>
  )
}
