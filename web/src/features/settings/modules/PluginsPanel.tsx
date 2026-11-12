import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { Info, PackagePlus, Plug, Power, PowerOff, Settings2, Trash2 } from 'lucide-react'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api, isApiError } from '@/api'
import { ConfirmDialog, errorMessage, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { SYSTEM_MANAGE } from '@/features/settings/lib/permissions'
import { PluginWebAdmission } from './plugins/PluginWebAdmission'
import { PluginConflictList } from './plugins/PluginConflictList'
import { PluginConsentDialog } from './plugins/PluginConsentDialog'
import { PluginDeclarationList } from './plugins/PluginDeclarationList'
import { PluginGrantStatus } from './plugins/PluginGrantStatus'
import { PluginSettingsDialog } from './plugins/PluginSettingsDialog'
import { PluginPackConsentDialog } from './plugins/PluginPackConsentDialog'
import { PluginPackResultDialog } from './plugins/PluginPackResultDialog'
import {
  PACKS_PATH,
  PLUGINS_PATH,
  pluginsKey,
  type InstalledPlugin,
  type PluginConflict,
  type PluginConsent,
  type PluginPackConsent,
  type PluginPackResult,
} from './plugins/types'
import { pluginManagementAllowed } from '@/kernel/modules/wasm_runtime'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

const PACKAGE_EXTENSION = '.aphelplugin'
const PACK_EXTENSION = '.aphelpack'

function groupByPack(plugins: InstalledPlugin[]) {
  const groups = new Map<string, { pack: InstalledPlugin['pack']; plugins: InstalledPlugin[] }>()
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

function reloadApp() {
  window.location.reload()
}

export function PluginsPanel() {
  const { t } = useTranslation('modules')
  return (
    <RequirePermission permission={SYSTEM_MANAGE}>
      {pluginManagementAllowed() ? (
        <PluginsPanelContent />
      ) : (
        <Page aria-label={t('plugins.title')}>
          <EmptyState icon={Plug} title={t('plugins.managementDisabledTitle')} description={t('plugins.managementDisabledDescription')} />
        </Page>
      )}
    </RequirePermission>
  )
}

function loadErrorDescription(error: unknown) {
  const message = errorMessage(error)
  return isApiError(error) ? `${message} (${error.status}: ${error.code})` : message
}

const CONFLICTS_UNACKNOWLEDGED = 'plugin_conflicts_unacknowledged'

function unacknowledgedConflicts(error: unknown): PluginConflict[] | null {
  if (!isApiError(error) || error.code !== CONFLICTS_UNACKNOWLEDGED) return null
  const details = error.details as { conflicts?: PluginConflict[] } | null
  return details?.conflicts ?? []
}

function PluginsPanelContent() {
  const { t } = useTranslation('modules')
  const queryClient = useQueryClient()
  const input = useRef<HTMLInputElement>(null)
  const [removing, setRemoving] = useState<InstalledPlugin | null>(null)
  const [configuring, setConfiguring] = useState<InstalledPlugin | null>(null)
  const [pending, setPending] = useState<{ file: File; consent: PluginConsent } | null>(null)
  const [details, setDetails] = useState<InstalledPlugin | null>(null)
  const [overlapping, setOverlapping] = useState<{ plugin: InstalledPlugin; conflicts: PluginConflict[]; webVersion?: string } | null>(null)
  const [admitting, setAdmitting] = useState<InstalledPlugin | null>(null)
  const [webAdmitted, setWebAdmitted] = useState(false)
  const [pendingPack, setPendingPack] = useState<{ file: File; pack: PluginPackConsent } | null>(null)
  const [packResult, setPackResult] = useState<PluginPackResult | null>(null)
  const readiness = useQuery({
    queryKey: [...pluginsKey, 'readiness'],
    queryFn: () => api.get<{ state: 'loading' | 'ready' | 'degraded'; version: number | null }>(`${PLUGINS_PATH}/readiness`),
    refetchInterval: (query) => query.state.data?.state === 'loading' ? 1000 : false,
  })
  const plugins = useQuery({
    queryKey: pluginsKey,
    enabled: readiness.data?.state === 'ready' || readiness.data?.state === 'degraded',
    queryFn: () => api.get<InstalledPlugin[]>(PLUGINS_PATH),
  })
  const changed = (message: string) => {
    notifySuccess(message)
    void queryClient.invalidateQueries({ queryKey: pluginsKey })
    window.setTimeout(reloadApp, 800)
  }
  const packageForm = (file: File, rolePresets: string[] = []) => {
    const form = new FormData()
    form.append('package', file)
    if (rolePresets.length > 0) form.append('role_presets', rolePresets.join(','))
    return form
  }
  const review = useMutation({
    mutationFn: (file: File) =>
      api
        .post<PluginConsent>(`${PLUGINS_PATH}?dry_run=true`, packageForm(file))
        .then((consent) => ({ file, consent })),
    onSuccess: (result) => setPending(result),
    onError: (error) => notifyError(error),
  })
  const reviewPack = useMutation({
    mutationFn: (file: File) =>
      api
        .post<PluginPackConsent>(`${PACKS_PATH}?dry_run=true`, packageForm(file))
        .then((pack) => ({ file, pack })),
    onSuccess: (result) => setPendingPack(result),
    onError: (error) => notifyError(error),
  })
  const installPack = useMutation({
    mutationFn: ({ file, rolePresets, acknowledge, webVersion }: { file: File; rolePresets: string[]; acknowledge: boolean; webVersion?: string }) =>
      api.post<PluginPackResult>(
        `${PACKS_PATH}?${new URLSearchParams({ acknowledge_conflicts: String(acknowledge), ...(webVersion ? { acknowledge_web_version: webVersion } : {}) })}`,
        packageForm(file, rolePresets),
      ),
    onSuccess: (result) => {
      setPendingPack(null)
      if (result.complete) changed(t('plugins.packInstalled', { name: result.name }))
      else {
        setPackResult(result)
        void queryClient.invalidateQueries({ queryKey: pluginsKey })
      }
    },
    onError: (error) => notifyError(error),
  })
  const install = useMutation({
    mutationFn: ({
      file,
      rolePresets,
      acknowledge,
      webVersion,
    }: {
      file: File
      rolePresets: string[]
      acknowledge: boolean
      webVersion?: string
    }) =>
      api.post<InstalledPlugin>(
        `${PLUGINS_PATH}?${new URLSearchParams({ acknowledge_conflicts: String(acknowledge), ...(webVersion ? { acknowledge_web_version: webVersion } : {}) })}`,
        packageForm(file, rolePresets),
      ),
    onSuccess: (response) => {
      setPending(null)
      changed(t('plugins.installed', { name: response.name }))
    },
    onError: (error) => notifyError(error),
  })
  const toggle = useMutation({
    mutationFn: ({ plugin, acknowledge = false, webVersion }: { plugin: InstalledPlugin; acknowledge?: boolean; webVersion?: string }) =>
      api.post(
        `${PLUGINS_PATH}/${plugin.key}/${plugin.enabled ? 'disable' : 'enable'}?${new URLSearchParams({ acknowledge_conflicts: String(acknowledge), ...(webVersion ? { acknowledge_web_version: webVersion } : {}) })}`,
      ),
    onSuccess: () => {
      setOverlapping(null)
      setAdmitting(null)
      changed(t('plugins.updated'))
    },
    onError: (error, { plugin, webVersion }) => {
      const conflicts = unacknowledgedConflicts(error)
      if (conflicts) setOverlapping({ plugin, conflicts, webVersion })
      else notifyError(error)
    },
  })
  const reapply = useMutation({
    mutationFn: (plugin: InstalledPlugin) => api.post<InstalledPlugin>(`${PLUGINS_PATH}/${plugin.key}/grants`),
    onSuccess: () => {
      notifySuccess(t('plugins.grantsReapplied'))
      void queryClient.invalidateQueries({ queryKey: pluginsKey })
    },
    onError: (error) => notifyError(error),
  })
  const uninstall = useMutation({
    mutationFn: (plugin: InstalledPlugin) => api.delete(`${PLUGINS_PATH}/${plugin.key}?purge=true`),
    onSuccess: () => {
      setRemoving(null)
      changed(t('plugins.removed'))
    },
    onError: (error) => notifyError(error),
  })

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
      <Button size="sm" onClick={() => input.current?.click()} disabled={busy}>
        <PackagePlus aria-hidden="true" />
        {review.isPending || reviewPack.isPending
          ? t('plugins.checking')
          : install.isPending || installPack.isPending
            ? t('plugins.uploading')
            : t('plugins.upload')}
      </Button>
    </>
  )

  return (
    <Page aria-label={t('plugins.title')}>
      <PageHeader level="section" title={t('plugins.title')} description={t('plugins.description')} actions={<>
        {upload}
      
      </>} />
      {readiness.isError ? (
        <ErrorState description={loadErrorDescription(readiness.error)} onRetry={() => void readiness.refetch()} />
      ) : readiness.isPending || readiness.data.state === 'loading' ? (
        <p role="status" className="text-sm text-muted-foreground">{t('plugins.catalogLoading')}</p>
      ) : plugins.isPending ? (
        <Skeleton className="h-32 w-full" />
      ) : plugins.isError ? (
        <ErrorState description={loadErrorDescription(plugins.error)} onRetry={() => void plugins.refetch()} />
      ) : plugins.data.length === 0 ? (
        <EmptyState icon={Plug} title={t('plugins.emptyTitle')} description={t('plugins.emptyDescription')} />
      ) : (
        <div className="grid gap-6">
          {groupByPack(plugins.data).map((group) => (
            <section key={group.pack?.key ?? 'loose'} className="grid gap-3" aria-label={group.pack ? group.pack.name : t('plugins.looseTitle')}>
              {(group.pack || plugins.data.some((plugin) => plugin.pack)) && (
                <div className="flex flex-wrap items-baseline gap-2">
                  <h3 className="text-base font-semibold">{group.pack ? group.pack.name : t('plugins.looseTitle')}</h3>
                  {group.pack && <span className="text-xs text-muted-foreground">{t('plugins.packLabel', { version: group.pack.version, count: group.plugins.length })}</span>}
                </div>
              )}
              <ul className="grid gap-3 md:grid-cols-2">
                {group.plugins.map((plugin) => (
                  <li key={plugin.key} className="flex h-80 flex-col gap-3 overflow-hidden rounded-md border bg-card p-4">
                    <div className="flex flex-wrap items-start justify-between gap-2">
                      <div className="grid gap-1">
                        <span className="font-semibold">{plugin.name}</span>
                        <span className="text-xs text-muted-foreground">
                          {plugin.key} · {t('version', { version: plugin.version })}
                        </span>
                      </div>
                      <div className="flex flex-wrap gap-1">
                        {plugin.backend_error && <Badge variant="destructive">{t('plugins.backendFailed')}</Badge>}
                        <Badge variant={plugin.enabled ? 'success' : 'secondary'}>
                          {plugin.enabled ? t('status.enabled') : t('status.disabled')}
                        </Badge>
                      </div>
                    </div>
                    {plugin.description && <p className="line-clamp-2 text-sm text-muted-foreground">{plugin.description}</p>}
                    <PluginDeclarationList plugin={plugin} compact />
                    <div className="mt-auto flex flex-wrap gap-2">
                      <Button size="sm" variant="outline" onClick={() => setDetails(plugin)}>
                        <Info aria-hidden="true" />
                        {t('plugins.viewDetails')}
                      </Button>
                      <Button size="sm" variant="outline" onClick={() => {
                        if (!plugin.enabled && plugin.has_web) { setWebAdmitted(false); setAdmitting(plugin) }
                        else toggle.mutate({ plugin })
                      }} disabled={toggle.isPending}>
                        {plugin.enabled ? <PowerOff aria-hidden="true" /> : <Power aria-hidden="true" />}
                        {plugin.enabled ? t('plugins.disable') : t('plugins.enable')}
                      </Button>
                      {plugin.settings.length > 0 && (
                        <Button size="sm" variant="outline" onClick={() => setConfiguring(plugin)}>
                          <Settings2 aria-hidden="true" />
                          {t('plugins.settings')}
                        </Button>
                      )}
                      <Button size="sm" variant="outline" onClick={() => setRemoving(plugin)}>
                        <Trash2 aria-hidden="true" />
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
      <Dialog open={details !== null} onOpenChange={(open) => { if (!open) setDetails(null) }}>
        {details && (
          <DialogContent size="xl">
            <DialogHeader>
              <DialogTitle>{t('plugins.detailsTitle', { name: details.name })}</DialogTitle>
              <DialogDescription>{details.key} · {t('version', { version: details.version })}</DialogDescription>
            </DialogHeader>
            {details.description && <p className="text-sm text-muted-foreground">{details.description}</p>}
            {details.backend_error && <SharedFieldError className="break-words">{details.backend_error}</SharedFieldError>}
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
            installPack.mutate({ file: pendingPack.file, rolePresets, acknowledge, webVersion: admitWeb ? pendingPack.pack.version : undefined })
          }
        />
      )}
      {packResult && <PluginPackResultDialog result={packResult} onClose={() => { setPackResult(null); reloadApp() }} />}
      <PluginConsentDialog
        key={pending ? `${pending.consent.key}@${pending.consent.version}` : "no-package"}
        consent={pending?.consent ?? null}
        pending={install.isPending}
        onCancel={() => setPending(null)}
        onConfirm={(rolePresets, acknowledge, admitWeb) => {
          if (pending) install.mutate({ file: pending.file, rolePresets, acknowledge, webVersion: admitWeb ? pending.consent.version : undefined })
        }}
      />
      <ConfirmDialog
        open={admitting !== null && overlapping === null}
        onOpenChange={(open) => { if (!open) setAdmitting(null) }}
        title={t('plugins.consentTitle', { name: admitting?.name ?? '', version: admitting?.version ?? '' })}
        confirmLabel={t('plugins.enable')}
        confirmDisabled={!webAdmitted}
        pending={toggle.isPending}
        onConfirm={() => { if (admitting && webAdmitted) toggle.mutate({ plugin: admitting, webVersion: admitting.version }) }}
      >
        <PluginWebAdmission checked={webAdmitted} onChange={setWebAdmitted} />
      </ConfirmDialog>
      <ConfirmDialog
        open={overlapping !== null}
        onOpenChange={(open) => {
          if (!open) { setOverlapping(null); setAdmitting(null) }
        }}
        title={t('plugins.toggleConflictsTitle', { name: overlapping?.plugin.name ?? '' })}
        confirmLabel={t('plugins.toggleConflictsConfirm')}
        pending={toggle.isPending}
        onConfirm={() => {
          if (overlapping) toggle.mutate({ plugin: overlapping.plugin, acknowledge: true, webVersion: overlapping.webVersion })
        }}
      >
        {overlapping && <PluginConflictList conflicts={overlapping.conflicts} />}
      </ConfirmDialog>
      {configuring && (
        <PluginSettingsDialog
          plugin={configuring}
          onClose={() => setConfiguring(null)}
          onSaved={() => queryClient.invalidateQueries({ queryKey: pluginsKey })}
        />
      )}
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => {
          if (!open) setRemoving(null)
        }}
        title={t('plugins.uninstallTitle', { name: removing?.name ?? '' })}
        description={t('plugins.uninstallDescription')}
        confirmLabel={t('plugins.uninstall')}
        variant="destructive"
        pending={uninstall.isPending}
        onConfirm={() => {
          if (removing) uninstall.mutate(removing)
        }}
      />
    </Page>
  )
}
