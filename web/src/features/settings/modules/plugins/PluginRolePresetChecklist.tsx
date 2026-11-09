import { useTranslation } from 'react-i18next'
import { Checkbox } from '@/components/ui/checkbox'
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
    <fieldset className="grid gap-2 rounded-md border p-3">
      <legend className="px-1 text-sm font-medium">{t('plugins.presetsTitle')}</legend>
      <p className="text-xs text-muted-foreground">{description ?? t('plugins.presetsDescription')}</p>
      {consent.role_presets.map((preset) => {
        const id = `plugin-preset-${preset.key}`
        const existing = exists(preset.key)
        return (
          <div key={preset.key} className="flex items-start gap-2">
            <Checkbox
              id={id}
              checked={existing || selected.includes(preset.key)}
              disabled={existing}
              onCheckedChange={(checked) =>
                onChange(
                  checked === true
                    ? [...selected, preset.key]
                    : selected.filter((key) => key !== preset.key),
                )
              }
            />
            <label htmlFor={id} className="grid gap-0.5 text-sm">
              <span className="font-medium">{preset.label}</span>
              <span className="text-xs text-muted-foreground">
                {existing
                  ? t('plugins.presetExists')
                  : (preset.description ?? t('plugins.presetPermissions', { count: preset.permissions.length }))}
              </span>
            </label>
          </div>
        )
      })}
    </fieldset>
  )
}
