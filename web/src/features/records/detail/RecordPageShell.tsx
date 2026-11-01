import type { ReactNode } from 'react'
import { Page } from '@/components/ui/page'
import { cn } from '@/lib/utils'

export interface RecordPageShellProps {
  header: ReactNode
  sidebar?: ReactNode
  children: ReactNode
  className?: string
}

export function RecordPageShell({ header, sidebar, children, className }: RecordPageShellProps) {
  return (
    <Page className={className}>
      {header}
      <div
        className={cn(
          'grid min-w-0 items-start gap-4 sm:gap-6',
          sidebar && 'lg:grid-cols-[minmax(0,1fr)_17rem] xl:grid-cols-[minmax(0,1fr)_20rem]',
        )}
      >
        <div className="min-w-0">{children}</div>
        {sidebar && <aside className="min-w-0 lg:sticky lg:top-20">{sidebar}</aside>}
      </div>
    </Page>
  )
}
