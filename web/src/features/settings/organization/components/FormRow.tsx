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
  const errorId = error ? `${id}-error` : undefined
  const hintId = hint ? `${id}-hint` : undefined
  const describedBy = [errorId, hintId].filter(Boolean).join(' ') || undefined
  return (
    <Field id={id} label={label}>
      {children(describedBy)}
      {error && (
        <p id={errorId} className="text-sm text-destructive">
          {error}
        </p>
      )}
      {hint && (
        <p id={hintId} className="text-xs text-muted-foreground">
          {hint}
        </p>
      )}
    </Field>
  )
}
