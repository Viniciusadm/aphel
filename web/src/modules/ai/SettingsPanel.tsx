import { useModuleTranslation } from '@/kernel/modules/i18n'
import { AI_KEY, useAiStatus } from './api'
import { DataDisclosure } from './DataDisclosure'

export function AiSettingsPanel() {
  const t = useModuleTranslation(AI_KEY)
  const status = useAiStatus()
  return (
    <div className="grid gap-2">
      <p className="text-sm text-muted-foreground">{t('settingsPanel.intro')}</p>
      <DataDisclosure status={status.data} compact />
    </div>
  )
}
