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
import { PluginConsentDialog } from './plugins/PluginConsentDialog'
import { PluginDeclarationList } from './plugins/PluginDeclarationList'
import { PluginGrantStatus } from './plugins/PluginGrantStatus'
import { PluginSettingsDialog } from './plugins/PluginSettingsDialog'
import { PLUGINS_PATH, pluginsKey, type InstalledPlugin, type PluginConsent } from './plugins/types'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

const PACKAGE_EXTENSION = '.aphelplugin'

function reloadApp() {
  window.location.reload()
}

export function PluginsPanel() {
  return (
    <RequirePermission permission={SYSTEM_MANAGE}>
      <PluginsPanelContent />
    </RequirePermission>
  )
}

function loadErrorDescription(error: unknown) {
  const message = errorMessage(error)
  return isApiError(error) ? `${message} (${error.status}: ${error.code})` : message
}

function PluginsPanelContent() {
  const { t } = useTranslation('modules')
  const queryClient = useQueryClient()
  const input = useRef<HTMLInputElement>(null)
  const [removing, setRemoving] = useState<InstalledPlugin | null>(null)
  const [configuring, setConfiguring] = useState<InstalledPlugin | null>(null)
  const [pending, setPending] = useState<{ file: File; consent: PluginConsent } | null>(null)
  const [details, setDetails] = useState<InstalledPlugin | null>(null)
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
  const install = useMutation({
    mutationFn: ({ file, rolePresets }: { file: File; rolePresets: string[] }) =>
      api.post<InstalledPlugin>(PLUGINS_PATH, packageForm(file, rolePresets)),
    onSuccess: (response) => {
      setPending(null)
      changed(t('plugins.installed', { name: response.name }))
    },
    onError: (error) => notifyError(error),
  })
  const toggle = useMutation({
    mutationFn: (plugin: InstalledPlugin) =>
      api.post(`${PLUGINS_PATH}/${plugin.key}/${plugin.enabled ? 'disable' : 'enable'}`),
    onSuccess: () => changed(t('plugins.updated')),
    onError: (error) => notifyError(error),
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

  const upload = (
    <>
      <input
        ref={input}
        type="file"
        accept={PACKAGE_EXTENSION}
        className="sr-only"
        aria-label={t('plugins.upload')}
        onChange={(event) => {
          const file = event.target.files?.[0]
          if (file) review.mutate(file)
          event.target.value = ''
        }}
      />
      <Button size="sm" onClick={() => input.current?.click()} disabled={review.isPending || install.isPending}>
        <PackagePlus aria-hidden="true" />
        {review.isPending ? t('plugins.checking') : install.isPending ? t('plugins.uploading') : t('plugins.upload')}
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
        <ul className="grid gap-3 md:grid-cols-2">
          {plugins.data.map((plugin) => (
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
                <Button size="sm" variant="outline" onClick={() => toggle.mutate(plugin)} disabled={toggle.isPending}>
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
      )}
      <Dialog open={details !== null} onOpenChange={(open) => { if (!open) setDetails(null) }}>
        {details && (
          <DialogContent className="max-w-2xl">
            <DialogHeader>
              <DialogTitle>{t('plugins.detailsTitle', { name: details.name })}</DialogTitle>
              <DialogDescription>{details.key} · {t('version', { version: details.version })}</DialogDescription>
            </DialogHeader>
            {details.description && <p className="text-sm text-muted-foreground">{details.description}</p>}
            {details.backend_error && <p role="alert" className="break-words text-sm text-destructive">{details.backend_error}</p>}
            <PluginGrantStatus plugin={details} pending={reapply.isPending} onReapply={() => reapply.mutate(details)} />
            <PluginDeclarationList plugin={details} />
          </DialogContent>
        )}
      </Dialog>
      <PluginConsentDialog
        consent={pending?.consent ?? null}
        pending={install.isPending}
        onCancel={() => setPending(null)}
        onConfirm={(rolePresets) => {
          if (pending) install.mutate({ file: pending.file, rolePresets })
        }}
      />
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
