import { Textarea } from '@/components/ui/textarea'
import { ariaProps, type FieldInputBaseProps } from './types'

export interface LongTextFieldInputProps extends FieldInputBaseProps<string> {
  maxLength?: number
  rows?: number
}

export function LongTextFieldInput({ maxLength, rows = 4, ...props }: LongTextFieldInputProps) {
  return (
    <Textarea
      {...ariaProps(props)}
      rows={rows}
      maxLength={maxLength}
      className={props.className}
      value={props.value ?? ''}
      onChange={(event) => props.onChange(event.target.value === '' ? null : event.target.value)}
    />
  )
}
