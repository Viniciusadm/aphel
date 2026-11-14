import { useTranslation } from 'react-i18next'
import { OptionCard } from '@/components/ui/option-card'
import { EMAIL_VISIBILITIES, type EmailVisibility } from '@/sdk'

interface VisibilityPickerProps {
  name: string
  value: EmailVisibility
  onChange: (value: EmailVisibility) => void
  disabled?: boolean
}

export function VisibilityPicker({ name, value, onChange, disabled }: VisibilityPickerProps) {
  const { t } = useTranslation('email')
  return (
    <div role="radiogroup" className="grid gap-2">
      {EMAIL_VISIBILITIES.map((visibility) => (
        <OptionCard
          key={visibility}
          type="radio"
          name={name}
          value={visibility}
          checked={value === visibility}
          disabled={disabled}
          onChange={() => onChange(visibility)}
          title={t(`visibility.${visibility}.label`)}
          description={t(`visibility.${visibility}.description`)}
        />
      ))}
    </div>
  )
}
