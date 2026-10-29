import type { ComponentProps } from 'react'
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { asAuthMessageKey } from '../lib/messages'

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
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <Input id={id} aria-invalid={errorKey ? true : undefined} aria-describedby={describedBy} {...inputProps} />
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
    </div>
  )
}
