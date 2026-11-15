import { ShieldAlert } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { ToggleField } from '@/components/ui/toggle-field'

export function PluginWebAdmission({ checked, onChange }: { checked: boolean; onChange: (value: boolean) => void }) {
  const { t } = useTranslation('modules')
  return (
    <section role="note" className="grid gap-3 rounded-md border border-warning/40 bg-warning/5 p-4">
      <div className="flex items-start gap-2">
        <ShieldAlert aria-hidden="true" className="mt-0.5 size-4 shrink-0 text-warning" />
        <div className="grid gap-1">
          <h3 className="text-sm font-semibold">{t('plugins.webPresent')}</h3>
          <p className="text-sm text-muted-foreground">{t('plugins.webWarning')}</p>
          <p className="text-xs text-muted-foreground">{t('plugins.webRecovery')}</p>
        </div>
      </div>
      <ToggleField label={t('plugins.webAdmission')} checked={checked} onCheckedChange={onChange} />
    </section>
  )
}
