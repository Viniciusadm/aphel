import type { ComponentProps } from 'react'
import { cn } from '@/lib/utils'

export type PageWidth = 'full' | 'form' | 'narrow'

const widthClasses: Record<PageWidth, string> = {
  full: '',
  form: 'max-w-4xl',
  narrow: 'max-w-2xl',
}

export interface PageProps extends ComponentProps<'section'> {
  width?: PageWidth
}

export function Page({ width = 'full', className, ...props }: PageProps) {
  return (
    <section
      data-slot="page"
      className={cn('flex w-full min-w-0 flex-col gap-6', widthClasses[width], className)}
      {...props}
    />
  )
}

export function PageSection({ className, ...props }: ComponentProps<'div'>) {
  return <div data-slot="page-section" className={cn('flex min-w-0 flex-col gap-4', className)} {...props} />
}
