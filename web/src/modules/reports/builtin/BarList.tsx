import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'

export interface BarListItem {
  key: string
  label: string
  value: number
  display: ReactNode
  hint?: ReactNode
}

export interface BarListProps {
  items: BarListItem[]
  label: string
  tone?: 'primary' | 'destructive'
}

export function BarList({ items, label, tone = 'primary' }: BarListProps) {
  const max = Math.max(1, ...items.map((item) => item.value))
  return (
    <ul aria-label={label} className="grid gap-3">
      {items.map((item) => (
        <li key={item.key} className="grid min-w-0 gap-1">
          <div className="flex min-w-0 items-baseline justify-between gap-3 text-sm">
            <span className="truncate font-medium">{item.label}</span>
            <span className="shrink-0 tabular-nums">{item.display}</span>
          </div>
          <div className="h-2 w-full overflow-hidden rounded-full bg-muted" aria-hidden>
            <div
              className={cn('h-full rounded-full', tone === 'primary' ? 'bg-primary' : 'bg-destructive')}
              style={{ width: `${(item.value / max) * 100}%` }}
            />
          </div>
          {item.hint && <p className="truncate text-xs text-muted-foreground">{item.hint}</p>}
        </li>
      ))}
    </ul>
  )
}
