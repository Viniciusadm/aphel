import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import {EMAIL_VISIBILITIES, type EmailVisibility} from '@/sdk'

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
        <label
          key={visibility}
          className={cn(
            'flex cursor-pointer items-start gap-3 rounded-md border p-3 text-sm transition-colors has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring',
            value === visibility ? 'border-primary-solid bg-accent/50' : 'hover:bg-accent/30',
            disabled && 'cursor-not-allowed opacity-60',
          )}
        >
          <input
            type="radio"
            name={name}
            value={visibility}
            checked={value === visibility}
            disabled={disabled}
            onChange={() => onChange(visibility)}
            className="mt-0.5 accent-current"
          />
          <span className="grid gap-0.5">
            <span className="font-medium">{t(`visibility.${visibility}.label`)}</span>
            <span className="text-muted-foreground">{t(`visibility.${visibility}.description`)}</span>
          </span>
        </label>
      ))}
    </div>
  )
}
