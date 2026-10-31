import type { ComponentProps, ReactNode } from 'react'
import { cn } from '@/lib/utils'
import { tabsListVariants, tabsTriggerVariants, type TabsSize } from './tabs-variants'

export interface SegmentedControlOption<T extends string> {
  value: T
  label: ReactNode
  icon?: ReactNode
  disabled?: boolean
  ariaLabel?: string
}

export interface SegmentedControlProps<T extends string> extends Omit<ComponentProps<'div'>, 'onChange'> {
  value: T
  onValueChange: (value: T) => void
  options: SegmentedControlOption<T>[]
  size?: TabsSize
}

export function SegmentedControl<T extends string>({
  value,
  onValueChange,
  options,
  size = 'md',
  className,
  ...props
}: SegmentedControlProps<T>) {
  return (
    <div data-slot="segmented-control" role="group" className={cn(tabsListVariants({ size }), className)} {...props}>
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          aria-pressed={option.value === value}
          aria-label={option.ariaLabel}
          disabled={option.disabled}
          onClick={() => onValueChange(option.value)}
          className={tabsTriggerVariants({ size })}
        >
          {option.icon}
          {option.label}
        </button>
      ))}
    </div>
  )
}
