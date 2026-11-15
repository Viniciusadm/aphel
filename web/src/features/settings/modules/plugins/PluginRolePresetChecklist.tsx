import { useTranslation } from 'react-i18next'
import { ToggleField } from '@/components/ui/toggle-field'
import type { PluginConsent } from './types'

interface PluginRolePresetChecklistProps {
  consent: Pick<PluginConsent, 'role_presets' | 'grants'>
  selected: string[]
  onChange: (selected: string[]) => void
  description?: string
}

export function PluginRolePresetChecklist({ consent, selected, onChange, description }: PluginRolePresetChecklistProps) {
  const { t } = useTranslation('modules')
  if (consent.role_presets.length === 0) return null
  const exists = (key: string) => consent.grants.some((status) => status.role === key && status.role_exists)
  return (
    <fieldset className="grid gap-3 rounded-md border p-4">
      <legend className="px-1 text-sm font-medium">{t('plugins.presetsTitle')}</legend>
      <p className="text-xs text-muted-foreground">{description ?? t('plugins.presetsDescription')}</p>
      {consent.role_presets.map((preset) => {
        const existing = exists(preset.key)
        return (
          <ToggleField
            key={preset.key}
            label={preset.label}
            description={existing ? t('plugins.presetExists') : (preset.description ?? t('plugins.presetPermissions', { count: preset.permissions.length }))}
            checked={existing || selected.includes(preset.key)}
            disabled={existing}
            onCheckedChange={(checked) => onChange(checked ? [...selected, preset.key] : selected.filter((key) => key !== preset.key))}
          />
        )
      })}
    </fieldset>
  )
}
