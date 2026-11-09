import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'

export interface KbdProps {
  children: ReactNode
  size?: 'sm' | 'md'
  className?: string
}

export function Kbd({ children, size = 'sm', className }: KbdProps) {
  return (
    <kbd
      className={cn(
        'inline-flex items-center justify-center rounded border bg-muted font-mono text-muted-foreground',
        size === 'sm' ? 'h-5 min-w-5 px-1 text-[11px]' : 'h-6 min-w-6 px-1.5 text-xs',
        className,
      )}
    >
      {children}
    </kbd>
  )
}

export interface KbdChordProps {
  steps: string[][]
  separator?: ReactNode
  size?: 'sm' | 'md'
  className?: string
}

export function KbdChord({ steps, separator, size, className }: KbdChordProps) {
  return (
    <span className={cn('inline-flex shrink-0 flex-wrap items-center gap-1', className)}>
      {steps.map((parts, index) => (
        <span key={`${index}:${parts.join('+')}`} className="inline-flex items-center gap-0.5">
          {index > 0 && separator !== undefined && <span className="px-0.5 text-xs text-muted-foreground">{separator}</span>}
          {parts.map((part) => (
            <Kbd key={part} size={size}>
              {part}
            </Kbd>
          ))}
        </span>
      ))}
    </span>
  )
}
