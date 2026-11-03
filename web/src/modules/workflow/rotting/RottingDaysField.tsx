import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { MAX_ROTTING_DAYS } from './lib'
import { Field } from '@/components/ui/field'

export interface RottingDaysFieldProps {
  value: string
  invalid?: boolean
  disabled?: boolean
  onChange: (value: string) => void
}

export function RottingDaysField({ value, invalid, disabled, onChange }: RottingDaysFieldProps) {
  const { t } = useTranslation('workflow', { keyPrefix: 'rotting' })

  return (
    <Field label={t('stage.label')} description={disabled ? t('stage.openOnly') : t('stage.description')} error={invalid ? t('stage.invalid') : undefined}>
      {(control) => <Input {...control} placeholder={t('placeholders.days')} type="number" min={1} max={MAX_ROTTING_DAYS} step={1} inputMode="numeric" value={value} disabled={disabled} onChange={(event) => onChange(event.target.value)} />}
    </Field>
  )
}
