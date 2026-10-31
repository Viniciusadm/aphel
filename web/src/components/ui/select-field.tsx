import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './select'

const EMPTY_VALUE = '__empty__'
const EMPTY_PLACEHOLDER = '—'

export interface SelectFieldOption {
  value: string
  label: string
}

export interface SelectFieldProps {
  id?: string
  value: string
  onValueChange: (value: string) => void
  options: SelectFieldOption[]
  emptyLabel?: string
  allowEmpty?: boolean
  className?: string
  'aria-label'?: string
}

export function SelectField({
  id,
  value,
  onValueChange,
  options,
  emptyLabel,
  allowEmpty = emptyLabel !== undefined,
  className,
  'aria-label': ariaLabel,
}: SelectFieldProps) {
  return (
    <Select
      value={value === '' ? EMPTY_VALUE : value}
      onValueChange={(next) => onValueChange(next === EMPTY_VALUE ? '' : next)}
    >
      <SelectTrigger id={id} className={className} aria-label={ariaLabel}>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {allowEmpty && <SelectItem value={EMPTY_VALUE}>{emptyLabel || EMPTY_PLACEHOLDER}</SelectItem>}
        {options.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            {option.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
