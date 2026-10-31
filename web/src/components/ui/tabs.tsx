import { Tabs as TabsPrimitive } from 'radix-ui'
import { createContext, useContext, type ComponentProps, type ReactNode } from 'react'
import { cn } from '@/lib/utils'
import { Badge } from './badge'
import { tabsListVariants, tabsTriggerVariants, type TabsSize } from './tabs-variants'


const TabsSizeContext = createContext<TabsSize>('md')

export const Tabs = TabsPrimitive.Root

export function TabsList({ className, size = 'md', ...props }: ComponentProps<typeof TabsPrimitive.List> & { size?: TabsSize }) {
  return (
    <TabsSizeContext.Provider value={size}>
      <TabsPrimitive.List data-slot="tabs-list" className={cn(tabsListVariants({ size }), className)} {...props} />
    </TabsSizeContext.Provider>
  )
}

export interface TabsTriggerProps extends ComponentProps<typeof TabsPrimitive.Trigger> {
  icon?: ReactNode
  count?: number
}

export function TabsTrigger({ className, icon, count, children, ...props }: TabsTriggerProps) {
  const size = useContext(TabsSizeContext)
  return (
    <TabsPrimitive.Trigger data-slot="tabs-trigger" className={cn(tabsTriggerVariants({ size }), className)} {...props}>
      {icon}
      {children}
      {count !== undefined && (
        <Badge tone="neutral" size="sm" count>
          {count}
        </Badge>
      )}
    </TabsPrimitive.Trigger>
  )
}

export function TabsContent({ className, ...props }: ComponentProps<typeof TabsPrimitive.Content>) {
  return <TabsPrimitive.Content data-slot="tabs-content" className={cn('outline-none', className)} {...props} />
}


