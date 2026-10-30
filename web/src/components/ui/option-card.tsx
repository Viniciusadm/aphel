import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'

interface OptionCardProps {
  type: 'radio' | 'checkbox'
  name: string
  value: string
  checked: boolean
  onChange: (checked: boolean) => void
  title: string
  description?: string
  badge?: ReactNode
  children?: ReactNode
  disabled?: boolean
}

export function OptionCard({
  type,
  name,
  value,
  checked,
  onChange,
  title,
  description,
  badge,
  children,
  disabled,
}: OptionCardProps) {
  return (
    <label
      className={cn(
        'flex cursor-pointer gap-3 rounded-lg border bg-card p-4 text-card-foreground transition-colors hover:bg-accent/50',
        'has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring has-[:focus-visible]:ring-offset-2 has-[:focus-visible]:ring-offset-background',
        checked && 'border-primary bg-primary/5',
        disabled && 'cursor-not-allowed opacity-60',
      )}
    >
      <input
        type={type}
        name={name}
        value={value}
        checked={checked}
        disabled={disabled}
        onChange={(event) => onChange(event.target.checked)}
        className="mt-0.5 size-4 shrink-0 accent-primary"
      />
      <span className="grid min-w-0 flex-1 gap-1">
        <span className="flex flex-wrap items-center gap-2 font-medium">
          {title}
          {badge}
        </span>
        {description && <span className="text-sm text-muted-foreground">{description}</span>}
        {children}
      </span>
    </label>
  )
}
