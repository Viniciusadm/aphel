import { Info, TriangleAlert } from 'lucide-react'
import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'

export function Notice({ children, className, tone = 'info' }: { children: ReactNode; className?: string; tone?: 'info' | 'warning' }) {
  const warning = tone === 'warning'
  const Icon = warning ? TriangleAlert : Info
  return (
    <div
      role="status"
      data-slot="notice"
      className={cn(
        'flex items-start gap-2 rounded-md border px-3 py-2 text-sm text-foreground',
        warning ? 'border-warning/40 bg-warning/5' : 'border-border bg-muted',
        className,
      )}
    >
      <Icon aria-hidden="true" className={cn('mt-0.5 size-4 shrink-0', warning ? 'text-warning' : 'text-muted-foreground')} />
      <div className="min-w-0">{children}</div>
    </div>
  )
}
