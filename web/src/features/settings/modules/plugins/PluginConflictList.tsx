import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import type { PluginConflict } from './types'

interface PluginConflictListProps {
  conflicts: PluginConflict[]
  acknowledged?: boolean
  onAcknowledge?: (value: boolean) => void
}

export function PluginConflictList({ conflicts, acknowledged = false, onAcknowledge }: PluginConflictListProps) {
  const { t } = useTranslation('modules')
  const id = useId()
  return (
    <section role="note" className="grid gap-2 rounded-md border border-warning/40 p-3">
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
      {onAcknowledge && (
        <div className="flex items-center gap-2">
          <Checkbox id={id} checked={acknowledged} onCheckedChange={(checked) => onAcknowledge(checked === true)} />
          <Label htmlFor={id} className="text-sm font-normal">
            {t('plugins.consentConflictsAck')}
          </Label>
        </div>
      )}
    </section>
  )
}
