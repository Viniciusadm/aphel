import { useTranslation } from 'react-i18next'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { REMINDER_OPTIONS, reminderLabel } from '../lib/recurrence'

export interface ReminderSelectProps {
  id?: string
  value: number | null
  onChange: (value: number | null) => void
  disabled?: boolean
  className?: string
}

const NONE_VALUE = 'none'

export function ReminderSelect({ id, value, onChange, disabled, className }: ReminderSelectProps) {
  const { t } = useTranslation('activities')
  const options = REMINDER_OPTIONS.includes(value) ? REMINDER_OPTIONS : [...REMINDER_OPTIONS, value]
  return (
    <NativeSelect
      id={id}
      className={className}
      disabled={disabled}
      value={value === null ? NONE_VALUE : String(value)}
      onChange={(event) => onChange(event.target.value === NONE_VALUE ? null : Number(event.target.value))}
    >
      {options.map((option) => (
        <option key={option ?? NONE_VALUE} value={option === null ? NONE_VALUE : String(option)}>
          {reminderLabel(option, t)}
        </option>
      ))}
    </NativeSelect>
  )
}
