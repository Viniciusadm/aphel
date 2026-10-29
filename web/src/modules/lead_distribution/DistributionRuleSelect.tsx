import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import { Label } from '@/components/ui/label'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useDistributionRules } from './api'
import type { DistributionRuleSelectProps } from './slots'

export function DistributionRuleSelect({ recordType, value, onChange, disabled }: DistributionRuleSelectProps) {
  const { t } = useTranslation('lead_distribution')
  const id = useId()
  const rules = useDistributionRules()
  const options = (rules.data ?? []).filter(
    (rule) => rule.record_type === recordType && (rule.is_active || rule.id === value),
  )
  const unknown = value && rules.isSuccess && !options.some((rule) => rule.id === value)

  return (
    <div className="grid gap-1.5">
      <Label htmlFor={id}>{t('ruleSelect.label')}</Label>
      <NativeSelect
        id={id}
        value={value ?? ''}
        disabled={disabled || rules.isPending}
        onChange={(event) => onChange(event.target.value || null)}
      >
        <option value="">{t('ruleSelect.none')}</option>
        {options.map((rule) => (
          <option key={rule.id} value={rule.id}>
            {rule.is_active ? rule.name : t('ruleSelect.inactive', { name: rule.name })}
          </option>
        ))}
        {unknown && <option value={value}>{t('ruleSelect.unknown')}</option>}
      </NativeSelect>
      <p className="text-xs text-muted-foreground">
        {rules.isSuccess && options.length === 0 ? t('ruleSelect.empty') : t('ruleSelect.hint')}
      </p>
    </div>
  )
}
