import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useId, type ReactNode } from 'react'
import { cn } from '@/lib/utils'
import { Label } from './label'

export interface FieldControlProps {
  id: string
  'aria-describedby': string | undefined
  'aria-invalid': boolean
}

export interface FieldProps {
  id?: string
  label: ReactNode
  description?: ReactNode
  error?: ReactNode
  required?: boolean
  hideLabel?: boolean
  labelAction?: ReactNode
  inline?: boolean
  'data-field'?: string
  className?: string
  children: ReactNode | ((control: FieldControlProps) => ReactNode)
}

export function Field({ id: providedId, label, description, error, required, hideLabel, labelAction, inline, 'data-field': dataField, className, children }: FieldProps) {
  const generatedId = useId()
  const id = providedId ?? generatedId
  const descriptionId = `${id}-description`
  const errorId = `${id}-error`
  const describedBy = [description ? descriptionId : null, error ? errorId : null].filter(Boolean).join(' ')

  return (
    <div data-slot="field" data-field={dataField} className={cn(inline ? 'flex flex-wrap items-center gap-2' : 'grid content-start gap-2', className)}>
      {(() => {
        const text = (
          <Label htmlFor={id} className={hideLabel ? 'sr-only' : undefined}>
            {label}
            {required && (
              <span aria-hidden className="text-destructive">
                *
              </span>
            )}
          </Label>
        )
        if (!labelAction) return text
        return (
          <div className="flex flex-wrap items-center justify-between gap-2">
            {text}
            {labelAction}
          </div>
        )
      })()}
      {typeof children === 'function'
        ? children({ id, 'aria-describedby': describedBy || undefined, 'aria-invalid': Boolean(error) })
        : children}
      {description && (
        <p id={descriptionId} className="text-sm text-muted-foreground">
          {description}
        </p>
      )}
      {error && (
        <SharedFieldError id={errorId}>
          {error}
        </SharedFieldError>
      )}
    </div>
  )
}
