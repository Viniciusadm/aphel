import { type ComponentProps } from 'react'
import { Input } from '@/components/ui/input'
import { Field } from '@/components/ui/field'

interface TextFieldProps extends Omit<ComponentProps<'input'>, 'id'> {
  label: string
  error?: string | null
  hint?: string
}

export function TextField({ label, error, hint, className, ...props }: TextFieldProps) {
  return (
    <Field label={label} error={error} description={!error ? hint : undefined}>
      {(control) => <Input {...props} {...control} className={className} />}
    </Field>
  )
}
