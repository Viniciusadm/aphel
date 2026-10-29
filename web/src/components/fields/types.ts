export interface FieldInputBaseProps<TValue> {
  id?: string
  value: TValue | null
  onChange: (value: TValue | null) => void
  onBlur?: () => void
  disabled?: boolean
  invalid?: boolean
  autoFocus?: boolean
  className?: string
  'aria-describedby'?: string
  'aria-labelledby'?: string
  'aria-label'?: string
}

export function ariaProps<TValue>(props: FieldInputBaseProps<TValue>) {
  return {
    id: props.id,
    disabled: props.disabled,
    autoFocus: props.autoFocus,
    onBlur: props.onBlur,
    'aria-invalid': props.invalid ? true : undefined,
    'aria-describedby': props['aria-describedby'],
    'aria-labelledby': props['aria-labelledby'],
    'aria-label': props['aria-label'],
  }
}
