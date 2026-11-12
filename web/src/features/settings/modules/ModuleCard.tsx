import { AlertTriangle, ArrowUpCircle, Download, MoreHorizontal, Puzzle, Settings2, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { useModulesApi, type ModuleInfo } from '@/kernel/modules/api'
import { useModuleSurface } from '@/kernel/modules/surface'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { disableChain, enableChain, moduleAnchor } from './chain'
import { notifyModuleError } from './errors'
import { ModuleRelations } from './ModuleRelations'
import { exportModuleData } from './export'
import { ModuleSettingsDialog } from './ModuleSettingsDialog'
import { serverViolations, type SettingViolation } from './form'
import { PurgeDataDialog } from './PurgeDataDialog'
import { useModuleText } from './text'
import { Alert } from '@/components/ui/alert'

type CompatibilityProblem = 'requires_aphel' | 'missing_dependency' | 'incompatible_dependency' | 'unknown'

const KNOWN_PROBLEMS: CompatibilityProblem[] = ['requires_aphel', 'missing_dependency', 'incompatible_dependency']

function compatibilityProblem(module: ModuleInfo): CompatibilityProblem | null {
  const status = module.compatibility.status
  if (status === 'compatible') return null
  return KNOWN_PROBLEMS.find((problem) => problem === status) ?? 'unknown'
}

const MAX_LISTED = 3

interface ModuleCardProps {
  module: ModuleInfo
  modules: ModuleInfo[]
  nameOf: (key: string) => string
  onChanged: () => Promise<unknown>
  showSurface?: boolean
}

export function ModuleCard({ module, modules, nameOf, onChanged, showSurface = true }: ModuleCardProps) {
  const { t } = useTranslation('modules')
  const modulesApi = useModulesApi()
  const text = useModuleText(module)
  const formatting = useRecordFormatting()
  const surface = useModuleSurface(module.key)
  const [configuring, setConfiguring] = useState<SettingViolation[] | null>(null)
  const [confirmDisable, setConfirmDisable] = useState(false)
  const [confirmEnable, setConfirmEnable] = useState(false)
  const [purging, setPurging] = useState(false)
  const [busy, setBusy] = useState(false)
  const name = text.name
  const problem = compatibilityProblem(module)
  const toEnable = enableChain(modules, module.key).filter((item) => item.key !== module.key)
  const toDisable = disableChain(modules, module.key).filter((item) => item.key !== module.key)
  const exists = (key: string) => modules.some((item) => item.key === key && !item.required)
  const pendingNames = (list: ModuleInfo[]) => list.map((item) => nameOf(item.key)).join(', ')
  const shortList = (items: string[]) =>
    items.length > MAX_LISTED ? t('adds.more', { items: items.slice(0, MAX_LISTED).join(', '), count: items.length - MAX_LISTED }) : items.join(', ')
  const date = (value: string) => formatDateTime(value, formatting)

  const enable = async () => {
    setBusy(true)
    try {
      for (const item of [...toEnable, module]) {
        try {
          await modulesApi.enable(item.key)
        } catch (error) {
          if (item.key === module.key && isApiError(error) && error.code === 'module_settings_required') {
            setConfiguring(serverViolations(error))
          }
          notifyModuleError(error, item.key === module.key ? undefined : nameOf(item.key))
          return
        }
      }
      notifySuccess(t('toast.enabled', { name }))
    } finally {
      setBusy(false)
      await onChanged()
    }
  }

  const disable = async () => {
    try {
      for (const item of [...toDisable, module]) {
        try {
          await modulesApi.disable(item.key)
        } catch (error) {
          notifyModuleError(error, item.key === module.key ? undefined : nameOf(item.key))
          return
        }
      }
      notifySuccess(t('toast.disabled', { name }))
    } finally {
      await onChanged()
    }
  }

  const exportData = async () => {
    setBusy(true)
    try {
      await exportModuleData(module.key, modulesApi)
      notifySuccess(t('toast.exported', { name }))
    } catch (error) {
      notifyModuleError(error)
    } finally {
      setBusy(false)
      await onChanged()
    }
  }

  return (
    <li className="contents">
      <Card id={moduleAnchor(module.key)} tabIndex={-1} className="scroll-mt-24 gap-4 py-4 outline-none">
        <CardHeader className="px-4">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="flex min-w-0 items-center gap-3">
              <span className="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted">
                <Puzzle className="size-5" aria-hidden="true" />
              </span>
              <div className="grid min-w-0 gap-0.5">
                <CardTitle className="break-words">{name}</CardTitle>
                {showSurface && (
                  <p className="text-xs text-muted-foreground">
                    {surface.hasScreens ? t('nature.screens') : t('nature.support')}
                  </p>
                )}
              </div>
            </div>
            <div className="flex flex-wrap gap-1.5">
              <Badge variant={module.enabled ? 'success' : 'secondary'}>
                {module.enabled ? t('status.enabled') : t('status.disabled')}
              </Badge>
              {module.update_available && (
                <Badge variant="warning">
                  <ArrowUpCircle aria-hidden="true" className="size-3" />
                  {t('updateAvailable')}
                </Badge>
              )}
            </div>
          </div>
          <CardDescription>{text.description}</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-3 px-4 text-sm">
          {problem && (
            <Alert
              role="status" className="flex gap-2 text-destructive"
            >
              <AlertTriangle className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
              <div className="grid gap-0.5">
                <p className="font-medium">{t(`compatibility.${problem}`)}</p>
                {module.compatibility.detail && <p className="break-words text-xs">{module.compatibility.detail}</p>}
                {problem === 'requires_aphel' && (
                  <p className="text-xs">{t('minAphelVersion', { version: module.min_aphel_version })}</p>
                )}
              </div>
            </Alert>
          )}
          {module.update_available && module.installed_version && (
            <p className="text-xs text-muted-foreground">
              {t('updateAvailableHint', { installed: module.installed_version, version: module.version })}{' '}
              <Button type="button" variant="link" size="sm" className="h-auto p-0 text-xs" disabled={busy} onClick={enable}>
                {t('applyUpdate')}
              </Button>
            </p>
          )}
          {showSurface && (
          <p className="text-xs text-muted-foreground">
            <span className="font-medium">{t('adds.title')}: </span>
            {surface.menu.length === 0 && surface.settings.length === 0
              ? t('adds.nothing')
              : [
                  surface.menu.length > 0 ? t('adds.menu', { items: shortList(surface.menu) }) : null,
                  surface.settings.length > 0 ? t('adds.settings', { items: shortList(surface.settings) }) : null,
                ]
                  .filter(Boolean)
                  .join(' · ')}
          </p>
          )}
          <ModuleRelations module={module} nameOf={nameOf} exists={exists} />
          <details className="text-xs">
            <summary className="cursor-pointer font-medium text-muted-foreground">{t('details.summary')}</summary>
            <div className="mt-2 grid gap-3">
              <p className="text-muted-foreground">
                {t('version', { version: module.version })} · <code>{module.key}</code>
              </p>
              <div className="grid gap-1">
                <h4 className="font-medium">{t('details.permissions')}</h4>
                {module.permissions.length === 0 ? (
                  <p className="text-muted-foreground">{t('details.noPermissions')}</p>
                ) : (
                  <ul className="grid gap-1">
                    {module.permissions.map((permission) => (
                      <li key={permission.permission} className="break-words">
                        <span>{text.permissionLabel(permission.permission, permission.label)}</span> <code className="text-muted-foreground">{permission.permission}</code>
                        {permission.record_scoped && (
                          <span className="text-muted-foreground"> · {t('details.recordScoped')}</span>
                        )}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
              <div className="grid gap-1">
                <h4 className="font-medium">{t('details.events')}</h4>
                {module.events.length === 0 ? (
                  <p className="text-muted-foreground">{t('details.noEvents')}</p>
                ) : (
                  <ul className="grid gap-1">
                    {module.events.map((event) => (
                      <li key={event.event_type} className="break-words">
                        <code>{event.event_type}</code>
                        {event.description && (
                          <span className="text-muted-foreground"> · {text.eventDescription(event.event_type, event.description)}</span>
                        )}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            </div>
          </details>
          <div className="grid gap-0.5 text-xs text-muted-foreground">
            {module.enabled && module.enabled_at && <p>{t('enabledAt', { date: date(module.enabled_at) })}</p>}
            {!module.enabled && module.disabled_at && <p>{t('disabledAt', { date: date(module.disabled_at) })}</p>}
            {module.data_exported_at && <p>{t('dataExportedAt', { date: date(module.data_exported_at) })}</p>}
            {module.data_purged_at && <p>{t('dataPurgedAt', { date: date(module.data_purged_at) })}</p>}
          </div>
          {module.held_by && <p className="text-xs text-warning">{t('plugins.heldByPlugin', { plugin: module.held_by })}</p>}
        </CardContent>
        <CardFooter className="mt-auto flex-wrap gap-2 px-4">
          {module.enabled ? (
            <Button size="sm" variant="outline" disabled={busy} onClick={() => setConfirmDisable(true)}>
              {t('actions.disable')}
            </Button>
          ) : (
            <Button
              size="sm"
              disabled={busy || problem !== null || Boolean(module.held_by)}
              aria-busy={busy}
              onClick={() => (toEnable.length > 0 ? setConfirmEnable(true) : void enable())}
            >
              {toEnable.length > 0 ? t('actions.enableWith', { names: pendingNames(toEnable) }) : t('actions.enable')}
            </Button>
          )}
          <Button size="sm" variant="outline" onClick={() => setConfiguring([])}>
            <Settings2 aria-hidden="true" />
            {t('actions.configure')}
          </Button>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button size="icon" variant="ghost" className="ml-auto size-8" aria-label={t('actions.more', { name })}>
                <MoreHorizontal aria-hidden="true" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem disabled={busy} onSelect={() => void exportData()}>
                <Download aria-hidden="true" />
                {t('actions.export')}
              </DropdownMenuItem>
              {!module.enabled && (
                <DropdownMenuItem variant="destructive" onSelect={() => setPurging(true)}>
                  <Trash2 aria-hidden="true" />
                  {t('actions.purge')}
                </DropdownMenuItem>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        </CardFooter>
      </Card>
      {configuring !== null && (
        <ModuleSettingsDialog
          module={module}
          initialViolations={configuring}
          onClose={() => setConfiguring(null)}
          onSaved={onChanged}
        />
      )}
      {purging && (
        <PurgeDataDialog module={module} name={name} onClose={() => setPurging(false)} onChanged={onChanged} />
      )}
      <ConfirmDialog
        open={confirmEnable}
        onOpenChange={setConfirmEnable}
        title={t('enableChain.title', { name })}
        description={t('enableChain.description', { name, names: pendingNames(toEnable) })}
        confirmLabel={t('enableChain.confirm')}
        onConfirm={enable}
      />
      <ConfirmDialog
        open={confirmDisable}
        onOpenChange={setConfirmDisable}
        title={t('disable.title', { name })}
        description={
          toDisable.length > 0
            ? t('disable.withDependents', { name, names: pendingNames(toDisable) })
            : t('disable.description')
        }
        confirmLabel={toDisable.length > 0 ? t('disable.confirmAll') : t('disable.confirm')}
        onConfirm={disable}
      />
    </li>
  )
}
