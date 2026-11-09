import { ChevronRight } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { FieldError } from '@/components/ui/field-error'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { PluginConflictList } from './PluginConflictList'
import { PluginDeclarationList } from './PluginDeclarationList'
import { PluginRolePresetChecklist } from './PluginRolePresetChecklist'
import { PluginWebAdmission } from './PluginWebAdmission'
import type { PackAction, PackPluginConsent, PluginPackConsent } from './types'

interface PluginPackConsentDialogProps {
  pack: PluginPackConsent
  pending: boolean
  onCancel: () => void
  onConfirm: (rolePresets: string[], acknowledgeConflicts: boolean, admitWeb: boolean) => void
}

const ACTION_TONE: Record<PackAction, 'primary' | 'warning' | 'neutral'> = {
  install: 'primary',
  upgrade: 'warning',
  unchanged: 'neutral',
}

function settingValue(value: unknown): string {
  if (typeof value === 'boolean') return value ? '✓' : '✗'
  return String(value)
}

export function PluginPackConsentDialog({ pack, pending, onCancel, onConfirm }: PluginPackConsentDialogProps) {
  const { t } = useTranslation('modules')
  const changing = pack.plugins.filter((plugin) => plugin.action !== 'unchanged')
  const consents = changing.flatMap((plugin) => (plugin.consent ? [plugin.consent] : []))
  const presetsSource = {
    role_presets: consents
      .flatMap((consent) => consent.role_presets)
      .filter((preset, index, all) => all.findIndex((other) => other.key === preset.key) === index),
    grants: consents
      .flatMap((consent) => consent.grants)
      .filter((grant, index, all) => all.findIndex((other) => other.role === grant.role) === index),
  }
  const [selected, setSelected] = useState<string[]>(pack.role_presets)
  const [acknowledged, setAcknowledged] = useState(false)
  const [webAdmitted, setWebAdmitted] = useState(false)
  const failing = pack.plugins.filter((plugin) => plugin.error)
  const conflicts = consents.flatMap((consent) => consent.conflicts)
  const blocked = consents.some((consent) => consent.module_conflicts.some((plan) => plan.blocked_by.length > 0))
  const needsWeb = changing.some((plugin) => plugin.consent?.has_web)
  const nothingToDo = changing.length === 0
  const title = pack.installed_version
    ? t('plugins.packConsentUpgradeTitle', { name: pack.name, from: pack.installed_version, version: pack.version })
    : t('plugins.packConsentTitle', { name: pack.name, version: pack.version })

  return (
    <Dialog open onOpenChange={(open) => { if (!open) onCancel() }}>
      <DialogContent size="xl">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{pack.description ?? t('plugins.packConsentDescription')}</DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 text-sm">
          {failing.length > 0 && <FieldError>{t('plugins.packHasErrors', { count: failing.length })}</FieldError>}
          {nothingToDo && <p>{t('plugins.packNothingChanges')}</p>}
          <section className="grid gap-2">
            <h3 className="font-medium">{t('plugins.packOrderTitle')}</h3>
            <p className="text-xs text-muted-foreground">{t('plugins.packOrderDescription')}</p>
            <ol className="grid gap-2">
              {pack.plugins.map((plugin, index) => (
                <PackPluginRow key={plugin.key} plugin={plugin} position={index + 1} />
              ))}
            </ol>
          </section>
          {needsWeb && <PluginWebAdmission checked={webAdmitted} onChange={setWebAdmitted} />}
          {conflicts.length > 0 && (
            <PluginConflictList conflicts={conflicts} acknowledged={acknowledged} onAcknowledge={setAcknowledged} />
          )}
          <PluginRolePresetChecklist
            consent={presetsSource}
            selected={selected}
            onChange={setSelected}
            description={t('plugins.packPresetsDescription')}
          />
        </div>
        <DialogFooter>
          <Button type="button" variant="ghost" onClick={onCancel}>
            {t('plugins.packCancel')}
          </Button>
          <Button
            type="button"
            disabled={
              pending ||
              nothingToDo ||
              failing.length > 0 ||
              blocked ||
              (conflicts.length > 0 && !acknowledged) ||
              (needsWeb && !webAdmitted)
            }
            onClick={() => onConfirm(selected, acknowledged, webAdmitted)}
          >
            {pending ? t('plugins.packInstalling') : t('plugins.packConfirm', { count: changing.length })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function PackPluginRow({ plugin, position }: { plugin: PackPluginConsent; position: number }) {
  const { t } = useTranslation('modules')
  const [open, setOpen] = useState(Boolean(plugin.error))
  const settings = plugin.settings
  return (
    <li className="rounded-md border">
      <button
        type="button"
        className="flex w-full items-center gap-3 px-3 py-2 text-left"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
      >
        <span className="w-5 text-right tabular-nums text-muted-foreground">{position}</span>
        <span className="flex-1">
          <span className="font-medium">{plugin.name}</span>
          <span className="ml-2 text-xs text-muted-foreground">
            {plugin.consent?.installed_version && plugin.action === 'upgrade'
              ? t('plugins.packFromTo', { from: plugin.consent.installed_version, to: plugin.version })
              : t('version', { version: plugin.version })}
          </span>
        </span>
        {plugin.error ? (
          <Badge tone="destructive">{t('plugins.packError')}</Badge>
        ) : (
          <Badge tone={ACTION_TONE[plugin.action]}>{t(`plugins.packActions.${plugin.action}`)}</Badge>
        )}
        <ChevronRight aria-hidden="true" className={open ? 'size-4 rotate-90 transition-transform' : 'size-4 transition-transform'} />
      </button>
      {open && (
        <div className="grid gap-3 border-t px-3 py-3">
          {plugin.error && (
            <div className="grid gap-1">
              <FieldError>{plugin.error.message}</FieldError>
              {plugin.error.fields.map((field) => (
                <p key={field} className="text-xs text-destructive">{field}</p>
              ))}
            </div>
          )}
          {settings.length > 0 && (
            <div className="grid gap-1">
              <p className="font-medium">{t('plugins.packSettingsTitle')}</p>
              <p className="text-xs text-muted-foreground">{t('plugins.packSettingsDescription')}</p>
              <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs">
                {settings.map((setting) => (
                  <div key={setting.key} className="contents">
                    <dt className="text-muted-foreground">{setting.label}</dt>
                    <dd>{settingValue(setting.value)}</dd>
                  </div>
                ))}
              </dl>
            </div>
          )}
          {plugin.consent && plugin.action !== 'unchanged' && <PluginDeclarationList plugin={plugin.consent} />}
          {plugin.action === 'unchanged' && <p className="text-muted-foreground">{t('plugins.packUnchangedDescription')}</p>}
        </div>
      )}
    </li>
  )
}
