import { useModuleTranslation } from '@/kernel/modules/i18n'
import type { ModuleSettingsPanelProps } from '@/kernel/modules/define'
import { OIDC_PANEL_STEPS } from './api'
import { ProviderSettingsPanel } from './shared/components'

export function OidcSettingsPanel({ moduleKey }: ModuleSettingsPanelProps) {
  const t = useModuleTranslation(moduleKey)
  const steps = Array.from({ length: OIDC_PANEL_STEPS }, (_, index) => t(`panel.steps.${index}`))
  return <ProviderSettingsPanel moduleKey={moduleKey} steps={steps} />
}
