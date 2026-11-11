import type { ReactNode } from 'react'

export function Notice({ children }: { children: ReactNode }) {
  return (
    <div role="status" className="rounded-md border border-border bg-muted px-3 py-2 text-sm text-foreground">
      {children}
    </div>
  )
}
