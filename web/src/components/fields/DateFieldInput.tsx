import { Input } from '@/components/ui/input'
import { ariaProps, type FieldInputBaseProps } from './types'

const DATE_PATTERN = /^\d{4}-\d{2}-\d{2}$/

export function DateFieldInput(props: FieldInputBaseProps<string>) {
  const value = typeof props.value === 'string' && DATE_PATTERN.test(props.value) ? props.value : ''
  return (
    <Input
      {...ariaProps(props)}
      type="date"
      className={props.className}
      value={value}
      onChange={(event) => props.onChange(event.target.value === '' ? null : event.target.value)}
    />
  )
}
