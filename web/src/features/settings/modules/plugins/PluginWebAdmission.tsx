import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'

export function PluginWebAdmission({ checked, onChange }: { checked: boolean; onChange: (value: boolean) => void }) {
  const { t } = useTranslation('modules')
  const id = useId()
  return (
    <section role="note" className="grid gap-2 rounded-md border border-warning/40 p-3">
      <p className="text-sm text-warning">{t('plugins.webWarning')}</p>
      <p className="text-xs text-muted-foreground">{t('plugins.webRecovery')}</p>
      <div className="flex items-center gap-2">
        <Checkbox id={id} checked={checked} onCheckedChange={(value) => onChange(value === true)} />
        <Label htmlFor={id} className="text-sm font-normal">{t('plugins.webAdmission')}</Label>
      </div>
    </section>
  )
}
