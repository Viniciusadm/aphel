import type { ComponentProps } from 'react'
import { cn } from '@/lib/utils'

export function SectionTitle({ className, ...props }: ComponentProps<'h2'>) {
  return <h2 data-slot="section-title" className={cn('text-base font-semibold', className)} {...props} />
}
