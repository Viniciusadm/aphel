import { useId } from 'react'
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
  const { t } = useTranslation('pipelines', { keyPrefix: 'rotting' })
  const descriptionId = useId()

  return (
    <Field label={t('stage.label')}>
      {(control) => <>
        <Input placeholder={t('placeholders.days')}
        {...control}
        type="number"
        min={1}
        max={MAX_ROTTING_DAYS}
        step={1}
        inputMode="numeric"
        value={value}
        disabled={disabled}
        aria-invalid={invalid || undefined}
        aria-describedby={descriptionId}
        onChange={(event) => onChange(event.target.value)}
      />
      <p id={descriptionId} className="text-xs text-muted-foreground">
        {disabled ? t('stage.openOnly') : t('stage.description')}
      </p>
      {invalid && <p className="text-sm text-destructive">{t('stage.invalid')}</p>}
      </>}
    </Field>
  )
}
