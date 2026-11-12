import type { ComponentProps, ReactNode } from 'react'
import { NativeSelect } from './native-select'
import { optionText } from './option-text'
import { defaultUiLabels } from './default-labels'
import { useUiLabels } from './labels-context'
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
  const labels = useUiLabels()
  const sectionLabel = labels.sectionNavigation ?? defaultUiLabels().sectionNavigation
  return (
    <>
    <NativeSelect className="w-full md:hidden" aria-label={props['aria-label'] ?? sectionLabel} value={value} onValueChange={(next) => onValueChange(next as T)}>
      {options.map((option) => <option key={option.value} value={option.value} disabled={option.disabled}>{option.ariaLabel ?? (optionText(option.label) || option.value)}</option>)}
    </NativeSelect>
    <div data-slot="segmented-control" role="group" className={cn(tabsListVariants({ size, variant: 'pill' }), className, 'hidden md:inline-flex')} {...props}>
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          aria-pressed={option.value === value}
          aria-label={option.ariaLabel}
          disabled={option.disabled}
          onClick={() => onValueChange(option.value)}
          className={tabsTriggerVariants({ size, variant: 'pill' })}
        >
          {option.icon}
          {option.label}
        </button>
      ))}
    </div>
    </>
  )
}
