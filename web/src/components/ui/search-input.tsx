import { Search, X } from 'lucide-react'
import type { ComponentProps } from 'react'
import { cn } from '@/lib/utils'
import { Input } from './input'

export interface SearchInputProps extends Omit<ComponentProps<typeof Input>, 'type' | 'onChange' | 'value'> {
  value: string
  onValueChange: (value: string) => void
  wrapperClassName?: string
  clearLabel?: string
}

export function SearchInput({ value, onValueChange, className, wrapperClassName, clearLabel, ...props }: SearchInputProps) {
  return (
    <div data-slot="search-input" className={cn('relative min-w-0', wrapperClassName)}>
      <Search
        aria-hidden
        className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
      />
      <Input
        type="search"
        value={value}
        onChange={(event) => onValueChange(event.target.value)}
        className={cn('pl-9 [&::-webkit-search-cancel-button]:hidden', clearLabel && 'pr-9', className)}
        {...props}
      />
      {clearLabel && value && (
        <button
          type="button"
          className="absolute top-1/2 right-2.5 flex size-5 -translate-y-1/2 items-center justify-center rounded-sm text-muted-foreground hover:text-foreground"
          aria-label={clearLabel}
          onClick={() => onValueChange('')}
        >
          <X className="size-3.5" aria-hidden />
        </button>
      )}
    </div>
  )
}
