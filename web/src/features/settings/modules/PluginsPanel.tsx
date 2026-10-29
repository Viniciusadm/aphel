import { PackagePlus, Plug, Power, PowerOff, Trash2 } from 'lucide-react'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { SectionHeader } from '@/features/settings/components/SectionHeader'

interface InstalledPlugin {
  key: string
  name: string
  version: string
  description: string | null
  enabled: boolean
  entities: string[]
  extended_types: string[]
  hooks: string[]
  events: string[]
  routes: string[]
  has_web: boolean
}

const PLUGINS_PATH = '/v1/plugins'
const pluginsKey = ['plugins'] as const
const PACKAGE_EXTENSION = '.aphelplugin'

function reloadApp() {
  window.location.reload()
}

export function PluginsPanel() {
  const { t } = useTranslation('modules')
  const queryClient = useQueryClient()
  const input = useRef<HTMLInputElement>(null)
  const [removing, setRemoving] = useState<InstalledPlugin | null>(null)
  const plugins = useQuery({
    queryKey: pluginsKey,
    queryFn: () => api.get<{ data: InstalledPlugin[] }>(PLUGINS_PATH).then((response) => response.data),
  })
  const changed = (message: string) => {
    notifySuccess(message)
    void queryClient.invalidateQueries({ queryKey: pluginsKey })
    window.setTimeout(reloadApp, 800)
  }
  const install = useMutation({
    mutationFn: (file: File) => {
      const form = new FormData()
      form.append('package', file)
      return api.post<{ data: InstalledPlugin }>(PLUGINS_PATH, form)
    },
    onSuccess: (response) => changed(t('plugins.installed', { name: response.data.name })),
    onError: (error) => notifyError(error),
  })
  const toggle = useMutation({
    mutationFn: (plugin: InstalledPlugin) =>
      api.post(`${PLUGINS_PATH}/${plugin.key}/${plugin.enabled ? 'disable' : 'enable'}`),
    onSuccess: () => changed(t('plugins.updated')),
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
          if (file) install.mutate(file)
          event.target.value = ''
        }}
      />
      <Button size="sm" onClick={() => input.current?.click()} disabled={install.isPending}>
        <PackagePlus aria-hidden="true" />
        {install.isPending ? t('plugins.uploading') : t('plugins.upload')}
      </Button>
    </>
  )

  return (
    <section className="flex flex-col gap-4" aria-label={t('plugins.title')}>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <SectionHeader title={t('plugins.title')} description={t('plugins.description')} />
        {upload}
      </div>
      {plugins.isPending ? (
        <Skeleton className="h-32 w-full" />
      ) : plugins.isError ? (
        <ErrorState onRetry={() => void plugins.refetch()} />
      ) : plugins.data.length === 0 ? (
        <EmptyState icon={Plug} title={t('plugins.emptyTitle')} description={t('plugins.emptyDescription')} />
      ) : (
        <ul className="grid gap-3 md:grid-cols-2">
          {plugins.data.map((plugin) => (
            <li key={plugin.key} className="grid gap-3 rounded-md border bg-card p-4">
              <div className="flex flex-wrap items-start justify-between gap-2">
                <div className="grid gap-1">
                  <span className="font-semibold">{plugin.name}</span>
                  <span className="text-xs text-muted-foreground">
                    {plugin.key} · {t('version', { version: plugin.version })}
                  </span>
                </div>
                <Badge variant={plugin.enabled ? 'success' : 'secondary'}>
                  {plugin.enabled ? t('status.enabled') : t('status.disabled')}
                </Badge>
              </div>
              {plugin.description && <p className="text-sm text-muted-foreground">{plugin.description}</p>}
              <dl className="grid gap-1 text-xs">
                {(
                  [
                    ['entities', plugin.entities],
                    ['extendedTypes', plugin.extended_types],
                    ['hooks', plugin.hooks],
                    ['events', plugin.events],
                    ['routes', plugin.routes],
                  ] as const
                )
                  .filter(([, values]) => values.length > 0)
                  .map(([label, values]) => (
                    <div key={label} className="flex flex-wrap gap-1">
                      <dt className="font-medium">{t(`plugins.declares.${label}`)}:</dt>
                      <dd className="text-muted-foreground">{values.join(', ')}</dd>
                    </div>
                  ))}
              </dl>
              <div className="flex flex-wrap gap-2">
                <Button size="sm" variant="outline" onClick={() => toggle.mutate(plugin)} disabled={toggle.isPending}>
                  {plugin.enabled ? <PowerOff aria-hidden="true" /> : <Power aria-hidden="true" />}
                  {plugin.enabled ? t('plugins.disable') : t('plugins.enable')}
                </Button>
                <Button size="sm" variant="outline" onClick={() => setRemoving(plugin)}>
                  <Trash2 aria-hidden="true" />
                  {t('plugins.uninstall')}
                </Button>
              </div>
            </li>
          ))}
        </ul>
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
    </section>
  )
}
