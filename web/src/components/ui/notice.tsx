import { Info } from 'lucide-react'
import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'

export function Notice({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div
      role="status"
      data-slot="notice"
      className={cn('flex items-start gap-2 rounded-md border border-border bg-muted px-3 py-2 text-sm text-foreground', className)}
    >
      <Info aria-hidden="true" className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0">{children}</div>
    </div>
  )
}
