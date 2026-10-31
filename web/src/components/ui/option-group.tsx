import { cva } from 'class-variance-authority'
import type { ComponentProps, ReactNode } from 'react'
import { cn } from '@/lib/utils'

const optionVariants = cva(
  'inline-flex items-center justify-center gap-2 rounded-full border bg-background px-4 text-sm outline-none transition-colors hover:bg-accent/50 focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 aria-pressed:border-primary aria-pressed:bg-primary/10 aria-pressed:font-medium aria-pressed:text-foreground [&>svg]:size-4 [&>svg]:shrink-0',
  {
    variants: {
      size: {
        sm: 'min-h-(--control-height-sm) px-3 text-xs',
        md: 'min-h-(--control-height-md)',
        touch: 'min-h-11',
        xl: 'min-h-14 min-w-24 px-5',
      },
    },
    defaultVariants: { size: 'md' },
  },
)

export interface OptionGroupOption<T extends string = string> {
  value: T
  label: ReactNode
  icon?: ReactNode
  disabled?: boolean
}

interface OptionGroupBaseProps<T extends string> extends Omit<ComponentProps<'div'>, 'onChange'> {
  options: OptionGroupOption<T>[]
  size?: 'sm' | 'md' | 'touch' | 'xl'
}

export interface OptionGroupSingleProps<T extends string> extends OptionGroupBaseProps<T> {
  multiple?: false
  value: T | ''
  onValueChange: (value: T | '') => void
  clearable?: boolean
}

export interface OptionGroupMultipleProps<T extends string> extends OptionGroupBaseProps<T> {
  multiple: true
  value: T[]
  onValueChange: (value: T[]) => void
  clearable?: never
}

export type OptionGroupProps<T extends string = string> = OptionGroupSingleProps<T> | OptionGroupMultipleProps<T>

export function OptionGroup<T extends string>({
  options,
  size = 'md',
  className,
  multiple,
  value,
  onValueChange,
  clearable,
  ...divProps
}: OptionGroupProps<T>) {
  const selected = (candidate: T) => (Array.isArray(value) ? value.includes(candidate) : value === candidate)
  const choose = (candidate: T) => {
    if (multiple) {
      const next = value.includes(candidate) ? value.filter((entry) => entry !== candidate) : [...value, candidate]
      onValueChange(next)
      return
    }
    onValueChange(clearable && value === candidate ? '' : candidate)
  }
  return (
    <div data-slot="option-group" role="group" className={cn('flex flex-wrap gap-2', className)} {...divProps}>
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          aria-pressed={selected(option.value)}
          disabled={option.disabled}
          onClick={() => choose(option.value)}
          className={optionVariants({ size })}
        >
          {option.icon}
          {option.label}
        </button>
      ))}
    </div>
  )
}
