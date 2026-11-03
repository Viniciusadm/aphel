import { useModuleTranslation } from '@/kernel/modules/i18n'
import { ProviderSettingsPanel } from '../sso/shared/components'
import type { ModuleSettingsPanelProps } from '@/kernel/modules/define'
import { MICROSOFT_PANEL_STEPS } from './api'

export function MicrosoftSettingsPanel({ moduleKey }: ModuleSettingsPanelProps) {
  const t = useModuleTranslation(moduleKey)
  const steps = Array.from({ length: MICROSOFT_PANEL_STEPS }, (_, index) => t(`panel.steps.${index}`))
  return <ProviderSettingsPanel moduleKey={moduleKey} steps={steps} />
}
