import { useId, type ComponentProps, type ReactNode } from 'react'
import { cn } from '@/lib/utils'
import { Checkbox } from './checkbox'
import { Label } from './label'
import { Switch } from './switch'

export interface ToggleFieldProps {
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
  type = 'checkbox',
  label,
  description,
  checked,
  onCheckedChange,
  disabled,
  className,
  controlProps,
}: ToggleFieldProps) {
  const id = useId()
  const Control = type === 'switch' ? Switch : Checkbox
  return (
    <div data-slot="toggle-field" className={cn('flex items-start gap-3', className)}>
      <Control
        id={id}
        checked={checked}
        disabled={disabled}
        onCheckedChange={(next: boolean | 'indeterminate') => onCheckedChange(next === true)}
        className={cn(type === 'checkbox' && 'mt-0.5')}
        {...(controlProps as object)}
      />
      <div className="grid gap-0.5">
        <Label htmlFor={id} className="font-normal">
          {label}
        </Label>
        {description && <p className="text-sm text-muted-foreground">{description}</p>}
      </div>
    </div>
  )
}
