import { ChevronDown } from 'lucide-react'
import type { ComponentProps } from 'react'
import { cn } from '@/lib/utils'

export interface NativeSelectProps extends ComponentProps<'select'> {
  onValueChange?: (value: string) => void
}

export function NativeSelect({ className, children, onChange, onValueChange, ...props }: NativeSelectProps) {
  return (
    <span className={cn('relative inline-flex min-w-0', className)}>
      <select
        className="h-(--control-height-md) w-full min-w-0 appearance-none rounded-(--input-radius) border border-input bg-background py-1 pr-8 pl-3 text-sm shadow-xs outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20"
        {...props}
        onChange={(event) => { onChange?.(event); onValueChange?.(event.target.value) }}
      >
        {children}
      </select>
      <ChevronDown
        className="pointer-events-none absolute top-1/2 right-2.5 size-4 -translate-y-1/2 text-muted-foreground"
        aria-hidden
      />
    </span>
  )
}
