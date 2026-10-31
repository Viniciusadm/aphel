import type { ComponentProps } from 'react'
import { cn } from '@/lib/utils'

export function FieldError({ className, ...props }: ComponentProps<'p'>) {
  return <p role="alert" {...props} className={cn('text-sm text-destructive', className)} />
}
