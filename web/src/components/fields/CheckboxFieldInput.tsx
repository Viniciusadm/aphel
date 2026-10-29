import { Checkbox } from '@/components/ui/checkbox'
import type { FieldInputBaseProps } from './types'

export function CheckboxFieldInput(props: FieldInputBaseProps<boolean>) {
  return (
    <Checkbox
      id={props.id}
      disabled={props.disabled}
      autoFocus={props.autoFocus}
      className={props.className}
      aria-invalid={props.invalid ? true : undefined}
      aria-describedby={props['aria-describedby']}
      aria-labelledby={props['aria-labelledby']}
      aria-label={props['aria-label']}
      checked={props.value === true}
      onBlur={props.onBlur}
      onCheckedChange={(checked) => props.onChange(checked === true)}
    />
  )
}
