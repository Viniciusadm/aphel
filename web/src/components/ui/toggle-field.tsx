import { useId, type ComponentProps, type ReactNode } from 'react'
import { cn } from '@/lib/utils'
import { Checkbox } from './checkbox'
import { Label } from './label'
import { Switch } from './switch'
import { FieldError } from './field-error'
import type { FieldControlProps } from './field'

export interface ToggleFieldProps {
  id?: string
  error?: ReactNode
  'data-field'?: string
  children?: (control: FieldControlProps) => ReactNode
  type?: 'checkbox' | 'switch'
  label: ReactNode
  description?: ReactNode
  checked: boolean
  onCheckedChange: (checked: boolean) => void
  disabled?: boolean
  className?: string
  controlProps?: Omit<ComponentProps<typeof Checkbox>, 'checked' | 'onCheckedChange' | 'disabled' | 'id'>
}

export function ToggleField({
  id: providedId,
  error,
  'data-field': dataField,
  children,
  type = 'checkbox',
  label,
  description,
  checked,
  onCheckedChange,
  disabled,
  className,
  controlProps,
}: ToggleFieldProps) {
  const generatedId = useId()
  const id = providedId ?? generatedId
  const descriptionId = `${id}-description`
  const errorId = `${id}-error`
  const control = { id, 'aria-invalid': Boolean(error), 'aria-describedby': [description ? descriptionId : null, error ? errorId : null].filter(Boolean).join(' ') || undefined }
  const Control = type === 'switch' ? Switch : Checkbox
  return (
    <div data-field={dataField} data-slot="toggle-field" className={cn('flex items-start gap-3', className)}>
      {children ? children(control) : <Control
        {...control}
        checked={checked}
        disabled={disabled}
        onCheckedChange={(next: boolean | 'indeterminate') => onCheckedChange(next === true)}
        className={cn(type === 'checkbox' && 'mt-0.5')}
        {...(controlProps as object)}
      />}
      <div className="grid gap-0.5">
        <Label htmlFor={id} className="font-normal">
          {label}
        </Label>
        {description && <p id={descriptionId} className="text-sm text-muted-foreground">{description}</p>}
        {error && <FieldError id={errorId}>{error}</FieldError>}
      </div>
    </div>
  )
}
