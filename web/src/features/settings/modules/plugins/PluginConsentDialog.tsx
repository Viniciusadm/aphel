import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog } from '@/components/feedback'
import { Notice } from '@/components/ui/notice'
import { PluginWebAdmission } from './PluginWebAdmission'
import { PluginDeclarationList } from './PluginDeclarationList'
import { PluginRolePresetChecklist } from './PluginRolePresetChecklist'
import { PluginConflictList } from './PluginConflictList'
import type { PluginConsent } from './types'

interface PluginConsentDialogProps {
  consent: PluginConsent | null
  pending: boolean
  onCancel: () => void
  onConfirm: (rolePresets: string[], acknowledgeConflicts: boolean, admitWeb: boolean) => void
}

export function PluginConsentDialog({ consent, pending, onCancel, onConfirm }: PluginConsentDialogProps) {
  const { t } = useTranslation('modules')
  const [presets, setPresets] = useState<{ key: string; selected: string[] }>({ key: '', selected: [] })
  const [acknowledged, setAcknowledged] = useState<{ key: string; value: boolean }>({ key: '', value: false })
  const [webAdmission, setWebAdmission] = useState<{ key: string; value: boolean }>({ key: '', value: false })
  const consentKey = consent ? `${consent.key}@${consent.version}` : ''
  const webAdmitted = webAdmission.key === consentKey && webAdmission.value
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
    !consent.has_web &&
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
      size="xl"
      description={t('plugins.consentDescription')}
      confirmLabel={t('plugins.consentConfirm')}
      pending={pending}
      confirmDisabled={Boolean(blockedModule) || (hasConflicts && !acknowledgedNow) || Boolean(consent?.has_web && !webAdmitted)}
      onConfirm={() => onConfirm(selected, acknowledgedNow, webAdmitted)}
    >
      {consent && (
        <div className="grid gap-4 text-sm">
          {consent.description && <p className="text-muted-foreground">{consent.description}</p>}
          {consent.has_web && <PluginWebAdmission checked={webAdmitted} onChange={(value) => setWebAdmission({ key: consentKey, value })} />}
          {asksNothing && <p>{t('plugins.consentNothing')}</p>}
          {consent.public_routes.length > 0 && (
            <Notice tone="warning">{t('plugins.consentPublicWarning')}</Notice>
          )}
          {consent.allowed_hosts.length > 0 && (
            <Notice tone="warning">{t('plugins.consentHttpWarning')}</Notice>
          )}
          {consent.module_conflicts.map((plan) => (
            <Notice key={plan.module} tone="warning">
              {plan.blocked_by.length > 0
                ? t('plugins.consentModuleBlocked', { name: plan.name, blockedBy: plan.blocked_by.join(', ') })
                : t('plugins.consentModuleDisables', { name: plan.name })}
            </Notice>
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
