import type { ComponentProps } from 'react'
import { cn } from '@/lib/utils'

export function PageTitle({ className, ...props }: ComponentProps<'h1'>) {
  return <h1 {...props} className={cn('min-w-0 text-xl font-semibold break-words tracking-tight [font-family:var(--font-family-heading)]', className)} />
}
