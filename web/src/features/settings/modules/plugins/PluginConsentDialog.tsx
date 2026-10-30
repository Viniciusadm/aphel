import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog } from '@/components/feedback'
import { PluginDeclarationList } from './PluginDeclarationList'
import { PluginRolePresetChecklist } from './PluginRolePresetChecklist'
import type { PluginConsent } from './types'

interface PluginConsentDialogProps {
  consent: PluginConsent | null
  pending: boolean
  onCancel: () => void
  onConfirm: (rolePresets: string[]) => void
}

export function PluginConsentDialog({ consent, pending, onCancel, onConfirm }: PluginConsentDialogProps) {
  const { t } = useTranslation('modules')
  const [presets, setPresets] = useState<{ key: string; selected: string[] }>({ key: '', selected: [] })
  const consentKey = consent ? `${consent.key}@${consent.version}` : ''
  const selected = presets.key === consentKey ? presets.selected : []
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
      onConfirm={() => onConfirm(selected)}
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
