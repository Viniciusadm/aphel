import type { ComponentProps } from 'react'
import { cn } from '@/lib/utils'

export function ItemList({ className, ...props }: ComponentProps<'ul'>) {
  return <ul data-slot="item-list" className={cn('grid gap-2 text-sm', className)} {...props} />
}

export function ItemRow({ className, ...props }: ComponentProps<'li'>) {
  return (
    <li
      data-slot="item-row"
      className={cn('flex flex-wrap items-center gap-2 rounded-md border bg-card p-3', className)}
      {...props}
    />
  )
}
