import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Info, PackagePlus, Plug, Power, PowerOff, Puzzle, Settings2, Trash2 } from 'lucide-react'
import { useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Navigate, useParams } from 'react-router'
import { isApiError } from '@/api'
import { ConfirmDialog } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { PageLoader } from '@/components/ui/page-loader'
import { Skeleton } from '@/components/ui/skeleton'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { FieldError } from '@/components/ui/field-error'
import { ToggleField } from '@/components/ui/toggle-field'
import { ModulesApiContext } from '@/kernel/modules/api'
import { ModuleCard } from '@/features/settings/modules/ModuleCard'
import { PluginGrantStatus } from '@/features/settings/modules/plugins/PluginGrantStatus'
import { PluginPackConsentDialog } from '@/features/settings/modules/plugins/PluginPackConsentDialog'
import { PluginPackResultDialog } from '@/features/settings/modules/plugins/PluginPackResultDialog'
import { PluginConflictList } from '@/features/settings/modules/plugins/PluginConflictList'
import { PluginConsentDialog } from '@/features/settings/modules/plugins/PluginConsentDialog'
import { PluginDeclarationList } from '@/features/settings/modules/plugins/PluginDeclarationList'
import { PluginSettingsDialog } from '@/features/settings/modules/plugins/PluginSettingsDialog'
import { PluginWebAdmission } from '@/features/settings/modules/plugins/PluginWebAdmission'
import type { PluginConflict, PluginPackConsent, PluginPackResult } from '@/features/settings/modules/plugins/types'
import { consoleApi, consoleKeys, tenantModulesApi, type TenantPlugin, type TenantPluginConsent } from '../api'
import { TenantStatusBadge } from '../components/TenantStatusBadge'
import { notifyConsoleError, notifyConsoleSuccess } from '../errors'
import { tenantPath, TENANTS_PATH } from '../paths'

const PACKAGE_EXTENSION = '.aphelplugin'
const PACK_EXTENSION = '.aphelpack'
const CONFLICTS_UNACKNOWLEDGED = 'plugin_conflicts_unacknowledged'

function unacknowledgedConflicts(error: unknown): PluginConflict[] | null {
  if (!isApiError(error) || error.code !== CONFLICTS_UNACKNOWLEDGED) return null
  const details = error.details as { conflicts?: PluginConflict[] } | null
  return details?.conflicts ?? []
}

function groupByPack(plugins: TenantPlugin[]) {
  const groups = new Map<string, { pack: TenantPlugin['pack']; plugins: TenantPlugin[] }>()
  for (const plugin of plugins) {
    const key = plugin.pack?.key ?? ''
    const group = groups.get(key) ?? { pack: plugin.pack, plugins: [] }
    group.plugins.push(plugin)
    groups.set(key, group)
  }
  return [...groups.values()].sort((left, right) => {
    if (!left.pack) return 1
    if (!right.pack) return -1
    return left.pack.name.localeCompare(right.pack.name)
  })
}

interface ToggleRequest {
  plugin: TenantPlugin
  acknowledge?: boolean
  webVersion?: string
}

export function TenantPluginsPage() {
  const { t } = useTranslation('console')
  const { t: tModules } = useTranslation('modules')
  const { slug = '' } = useParams()
  const queryClient = useQueryClient()
  const input = useRef<HTMLInputElement>(null)
  const [pending, setPending] = useState<{ file: File; consent: TenantPluginConsent } | null>(null)
  const [configuring, setConfiguring] = useState<TenantPlugin | null>(null)
  const [removing, setRemoving] = useState<TenantPlugin | null>(null)
  const [purge, setPurge] = useState(false)
  const [details, setDetails] = useState<TenantPlugin | null>(null)
  const [pendingPack, setPendingPack] = useState<{ file: File; pack: PluginPackConsent } | null>(null)
  const [packResult, setPackResult] = useState<PluginPackResult | null>(null)
  const [admitting, setAdmitting] = useState<TenantPlugin | null>(null)
  const [webAdmitted, setWebAdmitted] = useState(false)
  const [overlapping, setOverlapping] = useState<{ request: ToggleRequest; conflicts: PluginConflict[] } | null>(null)
  const tenant = useQuery({ queryKey: consoleKeys.tenant(slug), queryFn: () => consoleApi.tenant(slug), retry: false })
  const active = tenant.data?.status === 'active'
  const readiness = useQuery({
    queryKey: [...consoleKeys.plugins(slug), 'readiness'],
    queryFn: () => consoleApi.pluginsReadiness(slug),
    enabled: active,
    retry: false,
    refetchInterval: (query) => (query.state.data?.state === 'loading' ? 1000 : false),
  })
  const plugins = useQuery({
    queryKey: consoleKeys.plugins(slug),
    queryFn: () => consoleApi.plugins(slug),
    enabled: active && (readiness.data?.state === 'ready' || readiness.data?.state === 'degraded'),
    retry: false,
  })
  const modulesApi = useMemo(() => tenantModulesApi(slug), [slug])
  const modules = useQuery({
    queryKey: consoleKeys.modules(slug),
    queryFn: () => consoleApi.modules(slug),
    enabled: active,
    retry: false,
  })
  const optionalModules = modules.data?.filter((module) => !module.required) ?? []
  const moduleName = (key: string) => modules.data?.find((module) => module.key === key)?.name ?? key
  const refreshModules = () => queryClient.invalidateQueries({ queryKey: consoleKeys.modules(slug) })
  const refreshPlugins = () => queryClient.invalidateQueries({ queryKey: consoleKeys.plugins(slug) })

  const review = useMutation({
    mutationFn: (file: File) => consoleApi.reviewPlugin(slug, file).then((consent) => ({ file, consent })),
    onSuccess: setPending,
    onError: notifyConsoleError,
  })
  const install = useMutation({
    mutationFn: ({ rolePresets, acknowledge, webVersion }: { rolePresets: string[]; acknowledge: boolean; webVersion?: string }) => {
      if (!pending) throw new Error('no package to install')
      return consoleApi.installPlugin(slug, pending.file, rolePresets, acknowledge, webVersion)
    },
    onSuccess: (plugin) => {
      setPending(null)
      void refreshPlugins()
      notifyConsoleSuccess(t('plugins.installed', { name: plugin.name }))
    },
    onError: notifyConsoleError,
  })
  const toggle = useMutation({
    mutationFn: ({ plugin, acknowledge = false, webVersion }: ToggleRequest) =>
      consoleApi.setPluginEnabled(slug, plugin.key, !plugin.enabled, acknowledge, webVersion),
    onSuccess: () => {
      setOverlapping(null)
      setAdmitting(null)
      void refreshPlugins()
      notifyConsoleSuccess(t('plugins.updated'))
    },
    onError: (error, request) => {
      const conflicts = unacknowledgedConflicts(error)
      if (conflicts) setOverlapping({ request, conflicts })
      else notifyConsoleError(error)
    },
  })
  const uninstall = useMutation({
    mutationFn: (plugin: TenantPlugin) => consoleApi.uninstallPlugin(slug, plugin.key, purge),
    onSuccess: () => {
      setRemoving(null)
      void refreshPlugins()
      notifyConsoleSuccess(t('plugins.removed'))
    },
    onError: notifyConsoleError,
  })
  const reviewPack = useMutation({
    mutationFn: (file: File) => consoleApi.reviewPack(slug, file).then((pack) => ({ file, pack })),
    onSuccess: setPendingPack,
    onError: notifyConsoleError,
  })
  const installPack = useMutation({
    mutationFn: ({ rolePresets, acknowledge, webVersion }: { rolePresets: string[]; acknowledge: boolean; webVersion?: string }) => {
      if (!pendingPack) throw new Error('no pack to install')
      return consoleApi.installPack(slug, pendingPack.file, rolePresets, acknowledge, webVersion)
    },
    onSuccess: (result) => {
      setPendingPack(null)
      void refreshPlugins()
      if (result.complete) notifyConsoleSuccess(t('plugins.packInstalled', { name: result.name }))
      else setPackResult(result)
    },
    onError: notifyConsoleError,
  })
  const reapply = useMutation({
    mutationFn: (plugin: TenantPlugin) => consoleApi.reapplyPluginGrants(slug, plugin.key),
    onSuccess: (plugin) => {
      setDetails(plugin)
      void refreshPlugins()
      notifyConsoleSuccess(t('plugins.grantsReapplied'))
    },
    onError: notifyConsoleError,
  })

  if (tenant.isPending) return <PageLoader />
  if (tenant.isError && isApiError(tenant.error) && tenant.error.status === 404) return <Navigate to={TENANTS_PATH} replace />
  if (!tenant.data) return <ErrorState onRetry={() => void tenant.refetch()} />
  const data = tenant.data

  const startToggle = (plugin: TenantPlugin) => {
    if (!plugin.enabled && plugin.has_web) {
      setWebAdmitted(false)
      setAdmitting(plugin)
    } else {
      toggle.mutate({ plugin })
    }
  }
  const busy = review.isPending || install.isPending || reviewPack.isPending || installPack.isPending

  const upload = (
    <>
      <input
        ref={input}
        type="file"
        accept={`${PACKAGE_EXTENSION},${PACK_EXTENSION}`}
        className="sr-only"
        aria-label={t('plugins.upload')}
        onChange={(event) => {
          const file = event.target.files?.[0]
          if (file?.name.toLowerCase().endsWith(PACK_EXTENSION)) reviewPack.mutate(file)
          else if (file) review.mutate(file)
          event.target.value = ''
        }}
      />
      <Button size="sm" disabled={busy || !active} onClick={() => input.current?.click()}>
        <PackagePlus className="size-4" aria-hidden />
        {review.isPending || reviewPack.isPending ? t('plugins.checking') : install.isPending || installPack.isPending ? t('plugins.uploading') : t('plugins.upload')}
      </Button>
    </>
  )

  return (
    <Page>
      <PageHeader
        title={t('plugins.title', { name: data.name })}
        description={t('plugins.description')}
        badges={<TenantStatusBadge status={data.status} />}
        back={{ to: tenantPath(slug), label: data.name }}
        actions={upload}
      />
      {!active ? (
        <ErrorState title={t('plugins.unavailableTitle')} description={t('plugins.unavailableDescription')} />
      ) : (
        <div className="grid gap-8">
          {!data.plugin_management && (
            <p role="status" className="rounded-md border border-border bg-muted px-3 py-2 text-sm text-muted-foreground">
              {t('plugins.managementOff')}
            </p>
          )}
          <section className="grid gap-3" aria-label={t('plugins.pluginsHeading')}>
            <h2 className="text-base font-semibold">{t('plugins.pluginsHeading')}</h2>
            {readiness.isError ? (
              <ErrorState onRetry={() => void readiness.refetch()} />
            ) : readiness.isPending || readiness.data.state === 'loading' ? (
              <p role="status" className="text-sm text-muted-foreground">
                {t('plugins.catalogLoading')}
              </p>
            ) : plugins.isPending ? (
              <Skeleton className="h-32 w-full" />
            ) : plugins.isError ? (
              <ErrorState onRetry={() => void plugins.refetch()} />
            ) : plugins.data.length === 0 ? (
              <EmptyState icon={Plug} title={t('plugins.emptyTitle')} description={t('plugins.emptyDescription')} />
            ) : (
              <div className="grid gap-6">
                {groupByPack(plugins.data).map((group) => (
                  <section key={group.pack?.key ?? 'loose'} className="grid gap-3" aria-label={group.pack ? group.pack.name : t('plugins.looseTitle')}>
                    {(group.pack || plugins.data.some((plugin) => plugin.pack)) && (
                      <div className="flex flex-wrap items-baseline gap-2">
                        <h3 className="text-base font-semibold">{group.pack ? group.pack.name : t('plugins.looseTitle')}</h3>
                        {group.pack && (
                          <span className="text-xs text-muted-foreground">
                            {t('plugins.packLabel', { version: group.pack.version, count: group.plugins.length })}
                          </span>
                        )}
                      </div>
                    )}
                    <ul className="grid gap-3 md:grid-cols-2">
                      {group.plugins.map((plugin) => (
                        <li key={plugin.key} className="flex flex-col gap-3 rounded-md border bg-card p-4">
                          <div className="flex flex-wrap items-start justify-between gap-2">
                            <div className="grid gap-1">
                              <span className="font-semibold">{plugin.name}</span>
                              <span className="text-xs text-muted-foreground">
                                {plugin.key} · {t('plugins.version', { version: plugin.version })}
                              </span>
                            </div>
                            <div className="flex flex-wrap gap-1">
                              {plugin.backend_error && <Badge variant="destructive">{t('plugins.backendFailed')}</Badge>}
                              <Badge variant={plugin.enabled ? 'success' : 'secondary'}>
                                {plugin.enabled ? t('plugins.enabled') : t('plugins.disabled')}
                              </Badge>
                            </div>
                          </div>
                          {plugin.description && <p className="line-clamp-2 text-sm text-muted-foreground">{plugin.description}</p>}
                          <PluginDeclarationList plugin={plugin} compact />
                          <div className="mt-auto flex flex-wrap gap-2">
                            <Button size="sm" variant="outline" onClick={() => setDetails(plugin)}>
                              <Info className="size-4" aria-hidden />
                              {t('plugins.viewDetails')}
                            </Button>
                            <Button size="sm" variant="outline" disabled={toggle.isPending} onClick={() => startToggle(plugin)}>
                              {plugin.enabled ? <PowerOff className="size-4" aria-hidden /> : <Power className="size-4" aria-hidden />}
                              {plugin.enabled ? t('plugins.disable') : t('plugins.enable')}
                            </Button>
                            {plugin.settings.length > 0 && (
                              <Button size="sm" variant="outline" onClick={() => setConfiguring(plugin)}>
                                <Settings2 className="size-4" aria-hidden />
                                {t('plugins.settings')}
                              </Button>
                            )}
                            <Button
                              size="sm"
                              variant="outline"
                              onClick={() => {
                                setPurge(false)
                                setRemoving(plugin)
                              }}
                            >
                              <Trash2 className="size-4" aria-hidden />
                              {t('plugins.uninstall')}
                            </Button>
                          </div>
                        </li>
                      ))}
                    </ul>
                  </section>
                ))}
              </div>
            )}
          </section>
          <section className="grid gap-3" aria-label={t('plugins.modulesHeading')}>
            <div className="grid gap-1">
              <h2 className="text-base font-semibold">{t('plugins.modulesHeading')}</h2>
              <p className="text-sm text-muted-foreground">{t('plugins.modulesDescription')}</p>
            </div>
            {modules.isPending ? (
              <Skeleton className="h-56 w-full" />
            ) : modules.isError ? (
              <ErrorState onRetry={() => void modules.refetch()} />
            ) : optionalModules.length === 0 ? (
              <EmptyState icon={Puzzle} title={tModules('emptyTitle')} description={tModules('emptyDescription')} />
            ) : (
              <ModulesApiContext.Provider value={modulesApi}>
                <ul className="grid gap-3 md:grid-cols-2" aria-label={tModules('listLabel')}>
                  {optionalModules.map((module) => (
                    <ModuleCard key={module.key} module={module} nameOf={moduleName} onChanged={refreshModules} />
                  ))}
                </ul>
              </ModulesApiContext.Provider>
            )}
          </section>
        </div>
      )}
      <Dialog open={details !== null} onOpenChange={(open) => !open && setDetails(null)}>
        {details && (
          <DialogContent size="xl">
            <DialogHeader>
              <DialogTitle>{tModules('plugins.detailsTitle', { name: details.name })}</DialogTitle>
              <DialogDescription>
                {details.key} · {t('plugins.version', { version: details.version })}
              </DialogDescription>
            </DialogHeader>
            {details.description && <p className="text-sm text-muted-foreground">{details.description}</p>}
            {details.backend_error && <FieldError className="break-words">{details.backend_error}</FieldError>}
            <PluginGrantStatus plugin={details} pending={reapply.isPending} onReapply={() => reapply.mutate(details)} />
            <PluginDeclarationList plugin={details} />
          </DialogContent>
        )}
      </Dialog>
      {pendingPack && (
        <PluginPackConsentDialog
          key={`${pendingPack.pack.key}@${pendingPack.pack.version}`}
          pack={pendingPack.pack}
          pending={installPack.isPending}
          onCancel={() => setPendingPack(null)}
          onConfirm={(rolePresets, acknowledge, admitWeb) =>
            installPack.mutate({ rolePresets, acknowledge, webVersion: admitWeb ? pendingPack.pack.version : undefined })
          }
        />
      )}
      {packResult && <PluginPackResultDialog result={packResult} onClose={() => setPackResult(null)} />}
      <PluginConsentDialog
        key={pending ? `${pending.consent.key}@${pending.consent.version}` : 'no-package'}
        consent={pending?.consent ?? null}
        pending={install.isPending}
        onCancel={() => setPending(null)}
        onConfirm={(rolePresets, acknowledge, admitWeb) =>
          install.mutate({ rolePresets, acknowledge, webVersion: admitWeb && pending ? pending.consent.version : undefined })
        }
      />
      <ConfirmDialog
        open={admitting !== null && overlapping === null}
        onOpenChange={(open) => {
          if (!open) setAdmitting(null)
        }}
        title={tModules('plugins.consentTitle', { name: admitting?.name ?? '', version: admitting?.version ?? '' })}
        confirmLabel={t('plugins.enable')}
        confirmDisabled={!webAdmitted}
        pending={toggle.isPending}
        onConfirm={() => {
          if (admitting && webAdmitted) toggle.mutate({ plugin: admitting, webVersion: admitting.version })
        }}
      >
        <PluginWebAdmission checked={webAdmitted} onChange={setWebAdmitted} />
      </ConfirmDialog>
      <ConfirmDialog
        open={overlapping !== null}
        onOpenChange={(open) => {
          if (!open) {
            setOverlapping(null)
            setAdmitting(null)
          }
        }}
        title={tModules('plugins.toggleConflictsTitle', { name: overlapping?.request.plugin.name ?? '' })}
        confirmLabel={tModules('plugins.toggleConflictsConfirm')}
        pending={toggle.isPending}
        onConfirm={() => {
          if (overlapping) toggle.mutate({ ...overlapping.request, acknowledge: true })
        }}
      >
        {overlapping && <PluginConflictList conflicts={overlapping.conflicts} />}
      </ConfirmDialog>
      {configuring && (
        <PluginSettingsDialog
          plugin={configuring}
          onClose={() => setConfiguring(null)}
          onSaved={refreshPlugins}
          save={(values) => consoleApi.updatePluginSettings(slug, configuring.key, values)}
        />
      )}
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => {
          if (!open) setRemoving(null)
        }}
        title={tModules('plugins.uninstallTitle', { name: removing?.name ?? '' })}
        description={t('plugins.uninstallDescription')}
        confirmLabel={t('plugins.uninstall')}
        variant="destructive"
        pending={uninstall.isPending}
        onConfirm={() => {
          if (removing) uninstall.mutate(removing)
        }}
      >
        <ToggleField label={t('plugins.purge')} description={t('plugins.purgeHelp')} checked={purge} onCheckedChange={setPurge} />
      </ConfirmDialog>
    </Page>
  )
}
