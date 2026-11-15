import { useTranslation } from 'react-i18next'
import { ToggleField } from '@/components/ui/toggle-field'
import type { PluginConflict } from './types'

interface PluginConflictListProps {
  conflicts: PluginConflict[]
  acknowledged?: boolean
  onAcknowledge?: (value: boolean) => void
}

export function PluginConflictList({ conflicts, acknowledged = false, onAcknowledge }: PluginConflictListProps) {
  const { t } = useTranslation('modules')
  return (
    <section role="note" className="grid gap-2 rounded-md border border-warning/40 bg-warning/5 p-4">
      <h3 className="text-sm font-semibold text-warning">{t('plugins.consentConflictsTitle')}</h3>
      <p className="text-xs text-muted-foreground">{t('plugins.consentConflictsDescription')}</p>
      <ul className="grid gap-1 text-xs">
        {conflicts.map((conflict) => (
          <li key={`${conflict.other_plugin}:${conflict.kind}:${conflict.target}`}>
            {t('plugins.conflictWith', {
              kind: t(`plugins.conflictKinds.${conflict.kind}`, { target: conflict.target }),
              plugin: conflict.other_name,
            })}
          </li>
        ))}
      </ul>
      {onAcknowledge && <ToggleField label={t('plugins.consentConflictsAck')} checked={acknowledged} onCheckedChange={onAcknowledge} />}
    </section>
  )
}
