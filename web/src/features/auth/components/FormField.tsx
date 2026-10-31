import type { ComponentProps } from 'react'
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { asAuthMessageKey } from '../lib/messages'
import { Field } from '@/components/ui/field'

interface FormFieldProps extends ComponentProps<'input'> {
  id: string
  label: string
  error?: string
  hint?: string
}

export function FormField({ id, label, error, hint, ...inputProps }: FormFieldProps) {
  const { t } = useTranslation('auth')
  const errorKey = asAuthMessageKey(error)
  const hintId = hint ? `${id}-hint` : undefined
  const errorId = errorKey ? `${id}-error` : undefined
  const describedBy = [errorId, hintId].filter(Boolean).join(' ') || undefined

  return (
    <Field label={label}>
      {(control) => <>
        <Input {...control} aria-invalid={errorKey ? true : undefined} aria-describedby={describedBy} {...inputProps} />
      {errorKey && (
        <p id={errorId} className="text-sm text-destructive">
          {t(errorKey)}
        </p>
      )}
      {hint && !errorKey && (
        <p id={hintId} className="text-xs text-muted-foreground">
          {hint}
        </p>
      )}
      </>}
    </Field>
  )
}
