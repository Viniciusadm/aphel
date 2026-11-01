import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog } from '@/components/feedback'
import { PluginDeclarationList } from './PluginDeclarationList'
import { PluginRolePresetChecklist } from './PluginRolePresetChecklist'
import { PluginConflictList } from './PluginConflictList'
import type { PluginConsent } from './types'

interface PluginConsentDialogProps {
  consent: PluginConsent | null
  pending: boolean
  onCancel: () => void
  onConfirm: (rolePresets: string[], acknowledgeConflicts: boolean) => void
}

export function PluginConsentDialog({ consent, pending, onCancel, onConfirm }: PluginConsentDialogProps) {
  const { t } = useTranslation('modules')
  const [presets, setPresets] = useState<{ key: string; selected: string[] }>({ key: '', selected: [] })
  const [acknowledged, setAcknowledged] = useState<{ key: string; value: boolean }>({ key: '', value: false })
  const consentKey = consent ? `${consent.key}@${consent.version}` : ''
  const selected = presets.key === consentKey ? presets.selected : []
  const acknowledgedNow = acknowledged.key === consentKey && acknowledged.value
  const hasConflicts = (consent?.conflicts.length ?? 0) > 0
  const blockedModule = consent?.module_conflicts.find((plan) => plan.blocked_by.length > 0)
  const title = !consent
    ? ''
    : consent.installed_version
      ? t('plugins.consentUpgradeTitle', {
          name: consent.name,
          from: consent.installed_version,
          version: consent.version,
        })
      : t('plugins.consentTitle', { name: consent.name, version: consent.version })
  const asksNothing =
    consent !== null &&
    consent.requires.length === 0 &&
    consent.module_conflicts.length === 0 &&
    consent.conflicts.length === 0 &&
    consent.allowed_hosts.length === 0 &&
    consent.public_routes.length === 0
  return (
    <ConfirmDialog
      open={consent !== null}
      onOpenChange={(open) => {
        if (!open) onCancel()
      }}
      title={title}
      description={t('plugins.consentDescription')}
      confirmLabel={t('plugins.consentConfirm')}
      pending={pending}
      confirmDisabled={Boolean(blockedModule) || (hasConflicts && !acknowledgedNow)}
      onConfirm={() => onConfirm(selected, acknowledgedNow)}
    >
      {consent && (
        <div className="grid gap-3 text-sm">
          {consent.description && <p className="text-muted-foreground">{consent.description}</p>}
          {asksNothing && <p>{t('plugins.consentNothing')}</p>}
          {consent.public_routes.length > 0 && (
            <p role="note" className="text-warning">
              {t('plugins.consentPublicWarning')}
            </p>
          )}
          {consent.allowed_hosts.length > 0 && (
            <p role="note" className="text-warning">
              {t('plugins.consentHttpWarning')}
            </p>
          )}
          {consent.module_conflicts.map((plan) => (
            <p key={plan.module} role="note" className="text-warning">
              {plan.blocked_by.length > 0
                ? t('plugins.consentModuleBlocked', { name: plan.name, blockedBy: plan.blocked_by.join(', ') })
                : t('plugins.consentModuleDisables', { name: plan.name })}
            </p>
          ))}
          {hasConflicts && (
            <PluginConflictList
              conflicts={consent.conflicts}
              acknowledged={acknowledgedNow}
              onAcknowledge={(value) => setAcknowledged({ key: consentKey, value })}
            />
          )}
          <PluginRolePresetChecklist
            consent={consent}
            selected={selected}
            onChange={(next) => setPresets({ key: consentKey, selected: next })}
          />
          <PluginDeclarationList plugin={consent} />
        </div>
      )}
    </ConfirmDialog>
  )
}
