import { useId, type ReactNode } from 'react'
import { cn } from '@/lib/utils'
import { Label } from './label'

export interface FieldProps {
  label: string
  description?: string
  error?: string
  required?: boolean
  hideLabel?: boolean
  className?: string
  children: (control: { id: string; 'aria-describedby': string | undefined; 'aria-invalid': boolean }) => ReactNode
}

export function Field({ label, description, error, required, hideLabel, className, children }: FieldProps) {
  const id = useId()
  const descriptionId = `${id}-description`
  const errorId = `${id}-error`
  const describedBy = [description ? descriptionId : null, error ? errorId : null].filter(Boolean).join(' ')

  return (
    <div data-slot="field" className={cn('grid gap-2', className)}>
      <Label htmlFor={id} className={hideLabel ? 'sr-only' : undefined}>
        {label}
        {required && (
          <span aria-hidden className="text-destructive">
            *
          </span>
        )}
      </Label>
      {children({ id, 'aria-describedby': describedBy || undefined, 'aria-invalid': Boolean(error) })}
      {description && (
        <p id={descriptionId} className="text-sm text-muted-foreground">
          {description}
        </p>
      )}
      {error && (
        <p id={errorId} role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
    </div>
  )
}
