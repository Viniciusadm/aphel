import type { ComponentProps } from 'react'
import { NativeSelect } from './native-select'

export interface SelectFieldOption {
  value: string
  label: string
}

export interface SelectFieldProps extends Omit<ComponentProps<typeof NativeSelect>, 'value' | 'onChange' | 'children'> {
  value: string
  onValueChange: (value: string) => void
  options: SelectFieldOption[]
  emptyLabel?: string
  allowEmpty?: boolean
}

export function SelectField({ value, onValueChange, options, emptyLabel, allowEmpty = emptyLabel !== undefined, ...props }: SelectFieldProps) {
  return (
    <NativeSelect {...props} value={value} onChange={(event) => onValueChange(event.target.value)}>
      {(allowEmpty || value === '') && <option value="" disabled={!allowEmpty}>{emptyLabel || '—'}</option>}
      {options.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
    </NativeSelect>
  )
}
