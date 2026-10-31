import { NavLink } from 'react-router'
import type { ComponentProps, ReactNode } from 'react'
import { cn } from '@/lib/utils'
import { Badge } from './badge'
import { tabsListVariants, tabsTriggerVariants, type TabsSize } from './tabs-variants'

export function TabsNav({ className, size = 'md', ...props }: ComponentProps<'nav'> & { size?: TabsSize }) {
  return (
    <nav
      data-slot="tabs-nav"
      data-size={size}
      className={cn(tabsListVariants({ size }), className)}
      {...props}
    />
  )
}

export interface TabsNavItemProps extends Omit<ComponentProps<typeof NavLink>, 'children'> {
  size?: TabsSize
  icon?: ReactNode
  count?: number
  children?: ReactNode
}

export function TabsNavItem({ className, size = 'md', icon, count, children, ...props }: TabsNavItemProps) {
  return (
    <NavLink data-slot="tabs-nav-item" className={cn(tabsTriggerVariants({ size }), className as string)} {...props}>
      {icon}
      {children}
      {count !== undefined && (
        <Badge tone="neutral" size="sm" count>
          {count}
        </Badge>
      )}
    </NavLink>
  )
}
