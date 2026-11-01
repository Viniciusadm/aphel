import { Slot } from 'radix-ui'
import type { ComponentProps } from 'react'
import { cn } from '@/lib/utils'

export interface SidebarLinkProps extends ComponentProps<'a'> {
  asChild?: boolean
  collapsed?: boolean
  nested?: boolean
}

export function SidebarLink({ asChild, collapsed, nested, className, ...props }: SidebarLinkProps) {
  const Component = asChild ? Slot.Root : 'a'
  return (
    <Component
      {...props}
      className={cn(
        'relative flex h-(--control-height-md) items-center gap-3 rounded-md px-3 text-sm font-medium text-sidebar-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none aria-[current=page]:bg-sidebar-accent aria-[current=page]:text-sidebar-accent-foreground',
        collapsed && 'justify-center px-0',
        nested && !collapsed && 'pl-9',
        className,
      )}
    />
  )
}
