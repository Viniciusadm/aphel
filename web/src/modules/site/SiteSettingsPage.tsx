import { zodResolver } from '@hookform/resolvers/zod'
import { ArrowRight, ExternalLink, Eye, Plus, RefreshCw, Trash2 } from 'lucide-react'
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
  type SiteLink,
  type SiteMenuEntry,
  type SiteSettings,
  SortableList,
  SYSTEM_MANAGE,
  useAllMembers,
  useOrganizationConfiguration,
  useSaveConfiguration,
} from '@/sdk'
import {
  siteApi,
  useInvalidateSite,
  useSiteCustomers,
  useSiteOverview,
  useSitePathRedirects,
  useSiteRedirects,
  type SiteMenuItem,
  type SiteOverview,
} from './api'

const RESERVED_SEGMENTS = ['api']
const TAGLINE_MAX = 160
interface SiteModuleSettings extends SiteSettings {
  link_people: boolean
  people_owner: string | null
}

const DEFAULT_SETTINGS: SiteModuleSettings = {
  indexing: true,
  accounts: false,
  theme: null,
  tagline: null,
  menu: [],
  languages: [],
  link_people: false,
  people_owner: null,
  links: [],
  home: null,
}

const LINK_PREFIX = 'link.'
const LINK_LABEL_MAX = 60

function segmentOf(basePath: string): string {
  return basePath.replace(/^\/+|\/+$/g, '')
}

function basePathOf(segment: string): string {
  const trimmed = segment.trim().toLowerCase()
  return trimmed ? `/${trimmed}` : '/'
}

function currentSettings(configuration: OrganizationConfiguration): SiteModuleSettings {
  return { ...DEFAULT_SETTINGS, ...configuration.site }
}

function useSaveSite(configuration: OrganizationConfiguration) {
  const save = useSaveConfiguration()
  return (site: Partial<SiteModuleSettings>, extra: Record<string, unknown> = {}) =>
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
          <AppearanceCard key={JSON.stringify([configuration.data.site?.theme, configuration.data.site?.home, configuration.data.site?.tagline])} configuration={configuration.data} overview={overview.data} />
          <MenuCard key={overview.data.menu_items.map((item) => item.id).join()} configuration={configuration.data} overview={overview.data} />
          <LanguagesCard configuration={configuration.data} overview={overview.data} />
          <CustomersCard configuration={configuration.data} overview={overview.data} />
          <PathRedirectsCard />
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
  home: z.string(),
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
    control,
    handleSubmit,
    reset,
    formState: { errors, isSubmitting, isDirty },
  } = useForm<AppearanceValues>({
    resolver: zodResolver(appearanceSchema),
    defaultValues: { theme: settings.theme ?? '', home: settings.home ?? '', tagline: settings.tagline ?? '' },
  })
  const theme = useWatch({ control, name: 'theme' })
  const [previewing, setPreviewing] = useState(false)

  const preview = async () => {
    const tab = window.open('about:blank', '_blank')
    if (tab) tab.opener = null
    setPreviewing(true)
    try {
      const { url } = await siteApi.startPreview(theme || null)
      if (tab) tab.location.href = url
      else window.location.assign(url)
    } catch (error) {
      tab?.close()
      notifyError(error)
    } finally {
      setPreviewing(false)
    }
  }

  const onSubmit = handleSubmit(async (values) => {
    try {
      const saved = await saveSite({ theme: values.theme || null, home: values.home || null, tagline: values.tagline.trim() || null })
      reset({ theme: saved.site?.theme ?? '', home: saved.site?.home ?? '', tagline: saved.site?.tagline ?? '' })
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
                <div className="flex flex-wrap items-center gap-2">
                  <NativeSelect id="site-theme" className="min-w-48 flex-1" aria-describedby={describedBy} {...register('theme')}>
                    <option value="">{t('appearance.defaultTheme')}</option>
                    {overview.themes.map((option) => (
                      <option key={option.key} value={option.key}>
                        {option.name}
                      </option>
                    ))}
                  </NativeSelect>
                  <Button type="button" variant="outline" disabled={previewing} onClick={() => void preview()}>
                    <Eye aria-hidden="true" />
                    {t('appearance.preview')}
                  </Button>
                </div>
              )}
            </FormRow>
            {overview.home_candidates.length > 1 && (
              <FormRow id="site-home" label={t('appearance.home')} hint={t('appearance.homeHint')}>
                {(describedBy) => (
                  <NativeSelect id="site-home" className="min-w-48" aria-describedby={describedBy} {...register('home')}>
                    <option value="">{t('appearance.homeDefault', { name: overview.home_candidates[0].name })}</option>
                    {overview.home_candidates.map((candidate) => (
                      <option key={candidate.owner} value={candidate.owner}>
                        {candidate.name}
                      </option>
                    ))}
                  </NativeSelect>
                )}
              </FormRow>
            )}
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
  custom: boolean
}

function orderedMenu(items: SiteMenuItem[], entries: SiteMenuEntry[]): MenuRow[] {
  const position = (id: string) => {
    const index = entries.findIndex((entry) => entry.item === id)
    return index === -1 ? Number.MAX_SAFE_INTEGER : index
  }
  return items
    .map((item, index) => ({ item, index }))
    .sort((left, right) => position(left.item.id) - position(right.item.id) || left.index - right.index)
    .map(({ item }) => ({
      ...item,
      hidden: entries.some((entry) => entry.item === item.id && entry.hidden),
      custom: item.id.startsWith(LINK_PREFIX),
    }))
}

function menuItemsOf(overview: SiteOverview, links: SiteLink[]): SiteMenuItem[] {
  return [...overview.menu_items, ...links.map((link) => ({ id: `${LINK_PREFIX}${link.id}`, label: link.label, path: link.url }))]
}

const linkSchema = z.object({
  label: z.string().trim().min(1, 'errors.linkLabel').max(LINK_LABEL_MAX, 'errors.linkLabel'),
  url: z
    .string()
    .trim()
    .regex(/^(\/(?!\/)\S*|https?:\/\/\S+)$/, 'errors.redirectTarget'),
})

type LinkValues = z.infer<typeof linkSchema>

function newLinkId(): string {
  return crypto.randomUUID().replace(/-/g, '').slice(0, 12)
}

function MenuCard({ configuration, overview }: { configuration: OrganizationConfiguration; overview: SiteOverview }) {
  const { t } = useTranslation('site')
  const saveSite = useSaveSite(configuration)
  const [links, setLinks] = useState(() => currentSettings(configuration).links)
  const [rows, setRows] = useState(() => orderedMenu(menuItemsOf(overview, links), currentSettings(configuration).menu))
  const [saving, setSaving] = useState(false)
  const {
    register,
    handleSubmit,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<LinkValues>({ resolver: zodResolver(linkSchema), defaultValues: { label: '', url: '' } })
  const errorText = (message?: string) => (message?.startsWith('errors.') ? t(message as 'errors.linkLabel') : message)

  const persist = async (next: MenuRow[], nextLinks: SiteLink[] = links) => {
    const previous = { rows, links }
    setRows(next)
    setLinks(nextLinks)
    setSaving(true)
    try {
      await saveSite({ menu: next.map((row) => ({ item: row.id, hidden: row.hidden })), links: nextLinks })
      notifySuccess(t('menu.saved'))
      return true
    } catch (error) {
      setRows(previous.rows)
      setLinks(previous.links)
      notifyError(error)
      return false
    } finally {
      setSaving(false)
    }
  }

  const addLink = handleSubmit(async (values) => {
    const link = { id: newLinkId(), label: values.label.trim(), url: values.url.trim() }
    const added = await persist(
      [...rows, { id: `${LINK_PREFIX}${link.id}`, label: link.label, path: link.url, hidden: false, custom: true }],
      [...links, link],
    )
    if (added) reset({ label: '', url: '' })
  })

  const removeLink = (row: MenuRow) =>
    void persist(
      rows.filter((candidate) => candidate.id !== row.id),
      links.filter((link) => `${LINK_PREFIX}${link.id}` !== row.id),
    )

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('menu.title')}</CardTitle>
        <CardDescription>{t('menu.description')}</CardDescription>
      </CardHeader>
      <CardContent>
        {rows.length === 0 ? (
          <EmptyState title={t('menu.emptyWithLinks')} />
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
                {row.custom && (
                  <Button
                    size="sm"
                    variant="ghost"
                    disabled={saving}
                    onClick={() => removeLink(row)}
                    aria-label={t('menu.removeLink', { label: row.label })}
                  >
                    <Trash2 aria-hidden="true" />
                  </Button>
                )}
              </div>
            )}
          />
        )}
        <form className="mt-4 grid gap-4 border-t pt-4" onSubmit={addLink} noValidate>
          <p className="text-sm font-medium">{t('menu.addLinkTitle')}</p>
          <fieldset className="grid gap-4 sm:grid-cols-2" disabled={isSubmitting || saving}>
            <FormRow id="site-link-label" label={t('menu.linkLabel')} error={errorText(errors.label?.message)}>
              {(describedBy) => (
                <Input
                  id="site-link-label"
                  maxLength={LINK_LABEL_MAX}
                  placeholder={t('menu.linkLabelPlaceholder')}
                  aria-invalid={errors.label ? true : undefined}
                  aria-describedby={describedBy}
                  {...register('label')}
                />
              )}
            </FormRow>
            <FormRow
              id="site-link-url"
              label={t('menu.linkUrl')}
              error={errorText(errors.url?.message)}
              hint={t('menu.linkUrlHint')}
            >
              {(describedBy) => (
                <Input
                  id="site-link-url"
                  placeholder="/book/consulta"
                  autoComplete="off"
                  spellCheck={false}
                  inputMode="url"
                  aria-invalid={errors.url ? true : undefined}
                  aria-describedby={describedBy}
                  {...register('url')}
                />
              )}
            </FormRow>
          </fieldset>
          <div className="flex justify-end">
            <Button type="submit" variant="outline" disabled={isSubmitting || saving}>
              <Plus aria-hidden="true" />
              {t('menu.addLink')}
            </Button>
          </div>
        </form>
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

function PathRedirectsCard() {
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
