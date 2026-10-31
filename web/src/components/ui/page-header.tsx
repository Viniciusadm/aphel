import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'

export interface PageHeaderProps {
  title: string
  description?: string
  actions?: ReactNode
  className?: string
}

export function PageHeader({ title, description, actions, className }: PageHeaderProps) {
  return (
    <header data-slot="page-header" className={cn('flex flex-wrap items-start justify-between gap-4', className)}>
      <div className="grid gap-1">
        <h1 className="text-xl font-semibold tracking-tight [font-family:var(--font-family-heading)]">{title}</h1>
        {description && <p className="text-sm text-muted-foreground">{description}</p>}
      </div>
      {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
    </header>
  )
}
