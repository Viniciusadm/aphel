import { AlertTriangle, ArrowUpCircle, Download, MoreHorizontal, Puzzle, Settings2, Trash2 } from 'lucide-react'
import { useId, useState } from 'react'
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
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { notifyModuleError } from './errors'
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

interface ModuleCardProps {
  module: ModuleInfo
  nameOf: (key: string) => string
  onChanged: () => Promise<unknown>
}

export function ModuleCard({ module, nameOf, onChanged }: ModuleCardProps) {
  const { t } = useTranslation('modules')
  const modulesApi = useModulesApi()
  const text = useModuleText(module)
  const formatting = useRecordFormatting()
  const dependentsHintId = useId()
  const [configuring, setConfiguring] = useState<SettingViolation[] | null>(null)
  const [confirmDisable, setConfirmDisable] = useState(false)
  const [purging, setPurging] = useState(false)
  const [busy, setBusy] = useState(false)
  const name = text.name
  const problem = compatibilityProblem(module)
  const blockedByDependents = module.enabled && module.enabled_dependents.length > 0
  const dependentNames = module.enabled_dependents.map(nameOf).join(', ')
  const date = (value: string) => formatDateTime(value, formatting)

  const enable = async () => {
    setBusy(true)
    try {
      await modulesApi.enable(module.key)
      notifySuccess(t('toast.enabled', { name }))
    } catch (error) {
      if (isApiError(error) && error.code === 'module_settings_required') {
        setConfiguring(serverViolations(error))
      }
      notifyModuleError(error)
    } finally {
      setBusy(false)
      await onChanged()
    }
  }

  const disable = async () => {
    try {
      await modulesApi.disable(module.key)
      notifySuccess(t('toast.disabled', { name }))
    } catch (error) {
      notifyModuleError(error)
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
      <Card className="gap-4 py-4">
        <CardHeader className="px-4">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="flex min-w-0 items-center gap-3">
              <span className="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted">
                <Puzzle className="size-5" aria-hidden="true" />
              </span>
              <div className="grid min-w-0 gap-0.5">
                <CardTitle className="break-words">{name}</CardTitle>
                <p className="text-xs text-muted-foreground">
                  {t('version', { version: module.version })} · <code>{module.key}</code>
                </p>
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
          <div className="grid gap-1">
            <h4 className="text-xs font-medium text-muted-foreground">{t('dependencies.title')}</h4>
            {module.dependencies.length === 0 ? (
              <p className="text-xs text-muted-foreground">{t('dependencies.none')}</p>
            ) : (
              <ul className="flex flex-wrap gap-1.5">
                {module.dependencies.map((dependency) => {
                  const state = !dependency.installed ? 'missing' : dependency.enabled ? 'enabled' : 'disabled'
                  return (
                    <li key={dependency.key}>
                      <Badge variant={state === 'enabled' ? 'outline' : state === 'missing' ? 'destructive' : 'warning'}>
                        {nameOf(dependency.key)}
                        <span className="font-normal">
                          ({t('dependencies.requirement', { requirement: dependency.requirement })},{' '}
                          {t(`dependencies.state.${state}`)})
                        </span>
                      </Badge>
                    </li>
                  )
                })}
              </ul>
            )}
          </div>
          {module.enabled_dependents.length > 0 && (
            <div className="grid gap-1">
              <h4 className="text-xs font-medium text-muted-foreground">{t('dependents.title')}</h4>
              <ul className="flex flex-wrap gap-1.5">
                {module.enabled_dependents.map((key) => (
                  <li key={key}>
                    <Badge variant="outline">{nameOf(key)}</Badge>
                  </li>
                ))}
              </ul>
            </div>
          )}
          <details className="text-xs">
            <summary className="cursor-pointer font-medium text-muted-foreground">{t('details.summary')}</summary>
            <div className="mt-2 grid gap-3">
              <div className="grid gap-1">
                <h4 className="font-medium">{t('details.permissions')}</h4>
                {module.permissions.length === 0 ? (
                  <p className="text-muted-foreground">{t('details.noPermissions')}</p>
                ) : (
                  <ul className="grid gap-1">
                    {module.permissions.map((permission) => (
                      <li key={permission.permission} className="break-words">
                        <span>{permission.label}</span> <code className="text-muted-foreground">{permission.permission}</code>
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
                        {event.description && <span className="text-muted-foreground"> · {event.description}</span>}
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
          {blockedByDependents && (
            <p id={dependentsHintId} className="text-xs text-muted-foreground">
              {t('dependents.hint', { names: dependentNames })}
            </p>
          )}
        </CardContent>
        <CardFooter className="mt-auto flex-wrap gap-2 px-4">
          {module.enabled ? (
            <Button
              size="sm"
              variant="outline"
              disabled={busy || blockedByDependents}
              aria-describedby={blockedByDependents ? dependentsHintId : undefined}
              onClick={() => setConfirmDisable(true)}
            >
              {t('actions.disable')}
            </Button>
          ) : (
            <Button size="sm" disabled={busy || problem !== null || Boolean(module.held_by)} aria-busy={busy} onClick={() => void enable()}>
              {t('actions.enable')}
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
        open={confirmDisable}
        onOpenChange={setConfirmDisable}
        title={t('disable.title', { name })}
        description={t('disable.description')}
        confirmLabel={t('disable.confirm')}
        onConfirm={disable}
      />
    </li>
  )
}
