import { zodResolver } from '@hookform/resolvers/zod'
import { ArrowRight, ExternalLink, RefreshCw, Trash2 } from 'lucide-react'
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
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Skeleton } from '@/components/ui/skeleton'
import { ToggleField } from '@/components/ui/toggle-field'
import { cn } from '@/lib/utils'
import {
  FormRow,
  type OrganizationConfiguration,
  serverFieldErrors,
  SettingsNativeSelect as NativeSelect,
  SETTINGS_MANAGE,
  type SiteMenuEntry,
  type SiteSettings,
  SortableList,
  SYSTEM_MANAGE,
  useOrganizationConfiguration,
  useSaveConfiguration,
} from '@/sdk'
import { siteApi, useInvalidateSite, useSiteCustomers, useSiteOverview, useSiteRedirects, type SiteMenuItem, type SiteOverview } from './api'

const RESERVED_SEGMENTS = ['api']
const TAGLINE_MAX = 160
const DEFAULT_SETTINGS: SiteSettings = { indexing: true, accounts: false, theme: null, tagline: null, menu: [], languages: [] }

function segmentOf(basePath: string): string {
  return basePath.replace(/^\/+|\/+$/g, '')
}

function basePathOf(segment: string): string {
  const trimmed = segment.trim().toLowerCase()
  return trimmed ? `/${trimmed}` : '/'
}

function currentSettings(configuration: OrganizationConfiguration): SiteSettings {
  return { ...DEFAULT_SETTINGS, ...configuration.site }
}

function useSaveSite(configuration: OrganizationConfiguration) {
  const save = useSaveConfiguration()
  return (site: Partial<SiteSettings>, extra: Record<string, unknown> = {}) =>
    save({ ...extra, site: { ...currentSettings(configuration), ...site } })
}

export function SiteSettingsPage() {
  const { t } = useTranslation('site')
  const configuration = useOrganizationConfiguration()
  const overview = useSiteOverview()
  const loading = configuration.isPending || overview.isPending
  const failed = configuration.isError || overview.isError

  return (
    <Page aria-labelledby="site-settings-title">
      <PageHeader title={t('page.title')} description={t('page.description')} />
      {loading ? (
        <div className="grid max-w-3xl gap-4">
          <Skeleton className="h-40" />
          <Skeleton className="h-40" />
        </div>
      ) : failed || !configuration.data || !overview.data ? (
        <ErrorState onRetry={() => { void configuration.refetch(); void overview.refetch() }} />
      ) : (
        <div className="grid max-w-3xl gap-6">
          <SiteSummary overview={overview.data} />
          <AddressCard configuration={configuration.data} />
          <AppearanceCard key={JSON.stringify([configuration.data.site?.theme, configuration.data.site?.tagline])} configuration={configuration.data} overview={overview.data} />
          <MenuCard key={overview.data.menu_items.map((item) => item.id).join()} configuration={configuration.data} overview={overview.data} />
          <LanguagesCard configuration={configuration.data} overview={overview.data} />
          <CustomersCard configuration={configuration.data} overview={overview.data} />
          <RedirectsCard />
          <CacheCard />
        </div>
      )}
    </Page>
  )
}

function SiteSummary({ overview }: { overview: SiteOverview }) {
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
      indexing: z.boolean(),
    })
    .refine((values) => values.host.trim() !== '' || values.segment.trim().toLowerCase() !== appSegment, {
      path: ['segment'],
      message: 'errors.pathTaken',
    })

type AddressValues = z.infer<ReturnType<typeof addressSchema>>

function AddressCard({ configuration }: { configuration: OrganizationConfiguration }) {
  const { t } = useTranslation('site')
  const { hasPermission } = useSession()
  const canMove = hasPermission(SYSTEM_MANAGE)
  const saveSite = useSaveSite(configuration)
  const invalidate = useInvalidateSite()
  const appSegment = segmentOf(configuration.app_base_path)
  const defaults: AddressValues = {
    host: configuration.site_host ?? '',
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
  const publicBase = configuration.effective_public_base_url.replace(/\/+$/, '')
  const prefix = host.trim()
    ? `${publicBase.startsWith('https://') ? 'https' : 'http'}://${host.trim().toLowerCase()}/`
    : `${publicBase}/`
  const errorText = (message?: string) => (message?.startsWith('errors.') ? t(message as 'errors.path') : message)

  const onSubmit = handleSubmit(async (values) => {
    try {
      const saved = await saveSite(
        { indexing: values.indexing },
        canMove ? { site_base_path: basePathOf(values.segment), site_host: values.host.trim().toLowerCase() || null } : {},
      )
      reset({
        host: saved.site_host ?? '',
        segment: segmentOf(saved.site_base_path),
        indexing: saved.site?.indexing ?? true,
      })
      void invalidate()
      notifySuccess(t('saved'))
    } catch (error) {
      const fieldErrors = serverFieldErrors(error, {
        'mounts.site.base_path': 'segment',
        'mounts.site.host': 'host',
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
  theme: z.string(),
  tagline: z.string().trim().max(TAGLINE_MAX, 'errors.tagline'),
})

type AppearanceValues = z.infer<typeof appearanceSchema>

function AppearanceCard({ configuration, overview }: { configuration: OrganizationConfiguration; overview: SiteOverview }) {
  const { t } = useTranslation('site')
  const saveSite = useSaveSite(configuration)
  const invalidate = useInvalidateSite()
  const settings = currentSettings(configuration)
  const {
    register,
    handleSubmit,
    reset,
    formState: { errors, isSubmitting, isDirty },
  } = useForm<AppearanceValues>({
    resolver: zodResolver(appearanceSchema),
    defaultValues: { theme: settings.theme ?? '', tagline: settings.tagline ?? '' },
  })

  const onSubmit = handleSubmit(async (values) => {
    try {
      const saved = await saveSite({ theme: values.theme || null, tagline: values.tagline.trim() || null })
      reset({ theme: saved.site?.theme ?? '', tagline: saved.site?.tagline ?? '' })
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
            <FormRow id="site-theme" label={t('appearance.theme')} hint={t('appearance.themeHint')}>
              {(describedBy) => (
                <NativeSelect id="site-theme" aria-describedby={describedBy} {...register('theme')}>
                  <option value="">{t('appearance.defaultTheme')}</option>
                  {overview.themes.map((theme) => (
                    <option key={theme.key} value={theme.key}>
                      {theme.name}
                    </option>
                  ))}
                </NativeSelect>
              )}
            </FormRow>
            <FormRow
              id="site-tagline"
              label={t('appearance.tagline')}
              error={errors.tagline?.message ? t('errors.tagline') : undefined}
              hint={t('appearance.taglineHint')}
            >
              {(describedBy) => (
                <Input
                  id="site-tagline"
                  maxLength={TAGLINE_MAX}
                  placeholder={t('appearance.taglinePlaceholder')}
                  aria-invalid={errors.tagline ? true : undefined}
                  aria-describedby={describedBy}
                  {...register('tagline')}
                />
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

interface MenuRow extends SiteMenuItem {
  hidden: boolean
}

function orderedMenu(items: SiteMenuItem[], entries: SiteMenuEntry[]): MenuRow[] {
  const position = (id: string) => {
    const index = entries.findIndex((entry) => entry.item === id)
    return index === -1 ? Number.MAX_SAFE_INTEGER : index
  }
  return items
    .map((item, index) => ({ item, index }))
    .sort((left, right) => position(left.item.id) - position(right.item.id) || left.index - right.index)
    .map(({ item }) => ({ ...item, hidden: entries.some((entry) => entry.item === item.id && entry.hidden) }))
}

function MenuCard({ configuration, overview }: { configuration: OrganizationConfiguration; overview: SiteOverview }) {
  const { t } = useTranslation('site')
  const saveSite = useSaveSite(configuration)
  const [rows, setRows] = useState(() => orderedMenu(overview.menu_items, currentSettings(configuration).menu))
  const [saving, setSaving] = useState(false)

  const persist = async (next: MenuRow[]) => {
    const previous = rows
    setRows(next)
    setSaving(true)
    try {
      await saveSite({ menu: next.map((row) => ({ item: row.id, hidden: row.hidden })) })
      notifySuccess(t('menu.saved'))
    } catch (error) {
      setRows(previous)
      notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('menu.title')}</CardTitle>
        <CardDescription>{t('menu.description')}</CardDescription>
      </CardHeader>
      <CardContent>
        {rows.length === 0 ? (
          <EmptyState title={t('menu.empty')} />
        ) : (
          <SortableList
            items={rows}
            getId={(row) => row.id}
            getLabel={(row) => row.label}
            disabled={saving}
            label={t('menu.listLabel')}
            onReorder={(ids) => void persist(ids.map((id) => rows.find((row) => row.id === id)).filter((row): row is MenuRow => row !== undefined))}
            renderItem={(row, handle) => (
              <div className={cn('flex flex-wrap items-center gap-3 rounded-md border bg-card px-3 py-2', row.hidden && 'opacity-70')}>
                {handle}
                <div className="grid min-w-32 flex-1 gap-0.5">
                  <span className="truncate text-sm font-medium">{row.label}</span>
                  <code className="truncate text-xs text-muted-foreground">{row.path}</code>
                </div>
                <ToggleField
                  id={`site-menu-${row.id}`}
                  type="switch"
                  label={t('menu.shown')}
                  checked={!row.hidden}
                  disabled={saving}
                  onCheckedChange={(checked) =>
                    void persist(rows.map((candidate) => (candidate.id === row.id ? { ...candidate, hidden: checked !== true } : candidate)))
                  }
                />
              </div>
            )}
          />
        )}
      </CardContent>
    </Card>
  )
}

function languageName(code: string, locale: string): string {
  try {
    return new Intl.DisplayNames([locale], { type: 'language' }).of(code) ?? code
  } catch {
    return code
  }
}

function LanguagesCard({ configuration, overview }: { configuration: OrganizationConfiguration; overview: SiteOverview }) {
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

function CustomersCard({ configuration, overview }: { configuration: OrganizationConfiguration; overview: SiteOverview }) {
  const { t, i18n } = useTranslation('site')
  const saveSite = useSaveSite(configuration)
  const invalidate = useInvalidateSite()
  const [search, setSearch] = useState('')
  const [saving, setSaving] = useState(false)
  const enabled = currentSettings(configuration).accounts || overview.customers_required
  const customers = useSiteCustomers(search.trim())
  const formatDate = (value: string | null) =>
    value ? new Intl.DateTimeFormat(i18n.language, { dateStyle: 'short' }).format(new Date(value)) : '—'

  const toggle = async (checked: boolean) => {
    setSaving(true)
    try {
      await saveSite({ accounts: checked })
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
          onCheckedChange={(checked) => void toggle(checked === true)}
        />
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

function RedirectsCard() {
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

function CacheCard() {
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
