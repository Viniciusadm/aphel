import { LoaderCircle } from 'lucide-react'
import type { ComponentProps } from 'react'
import { cn } from '@/lib/utils'

export function Spinner({ className, ...props }: ComponentProps<typeof LoaderCircle>) {
  return <LoaderCircle data-slot="spinner" aria-hidden className={cn('size-4 animate-spin', className)} {...props} />
}
