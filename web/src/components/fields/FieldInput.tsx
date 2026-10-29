import { useFieldTypeRenderers } from '@/kernel/modules/field-types'
import type { FieldDefinition } from '@/features/records/types'

export interface FieldInputProps {
  field: FieldDefinition
  value: unknown
  onChange: (value: unknown) => void
  initialValue?: unknown
  id?: string
  onBlur?: () => void
  disabled?: boolean
  invalid?: boolean
  autoFocus?: boolean
  className?: string
  'aria-describedby'?: string
  'aria-labelledby'?: string
  'aria-label'?: string
}

export function FieldInput(props: FieldInputProps) {
  const { Input } = useFieldTypeRenderers(props.field.field_type, props.field.key)
  return Input ? <Input {...props} /> : null
}
