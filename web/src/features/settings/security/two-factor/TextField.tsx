import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useId, type ComponentProps } from 'react'
import { Input } from '@/components/ui/input'
import { Field } from '@/components/ui/field'

interface TextFieldProps extends Omit<ComponentProps<'input'>, 'id'> {
  label: string
  error?: string | null
  hint?: string
}

export function TextField({ label, error, hint, className, ...props }: TextFieldProps) {
  const id = useId()
  const errorId = `${id}-error`
  const hintId = `${id}-hint`
  const describedBy = [error ? errorId : null, hint && !error ? hintId : null].filter(Boolean).join(' ') || undefined

  return (
    <Field label={label}>
      {(control) => <>
        <Input
        {...control}
        className={className}
        aria-invalid={error ? true : undefined}
        aria-describedby={describedBy}
        {...props}
      />
      {error && (
        <SharedFieldError id={errorId}>
          {error}
        </SharedFieldError>
      )}
      {hint && !error && (
        <p id={hintId} className="text-xs text-muted-foreground">
          {hint}
        </p>
      )}
      </>}
    </Field>
  )
}
