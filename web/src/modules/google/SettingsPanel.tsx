import { useModuleTranslation } from '@/kernel/modules/i18n'
import { ProviderSettingsPanel } from '../sso/shared/components'
import type { ModuleSettingsPanelProps } from '@/kernel/modules/define'
import { GOOGLE_PANEL_STEPS } from './api'

export function GoogleSettingsPanel({ moduleKey }: ModuleSettingsPanelProps) {
  const t = useModuleTranslation(moduleKey)
  const steps = Array.from({ length: GOOGLE_PANEL_STEPS }, (_, index) => t(`panel.steps.${index}`))
  return <ProviderSettingsPanel moduleKey={moduleKey} steps={steps} />
}
