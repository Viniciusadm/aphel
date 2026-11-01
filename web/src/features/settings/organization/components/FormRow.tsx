import type { ReactNode } from 'react'
import { Field } from '@/components/ui/field'

interface FormRowProps {
  id: string
  label: string
  error?: string
  hint?: string
  children: (describedBy: string | undefined) => ReactNode
}

export function FormRow({ id, label, error, hint, children }: FormRowProps) {
  return (
    <Field id={id} label={label} error={error} description={hint}>
      {(control) => children(control['aria-describedby'])}
    </Field>
  )
}
