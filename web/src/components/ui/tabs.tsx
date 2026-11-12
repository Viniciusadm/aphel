import { Tabs as TabsPrimitive } from 'radix-ui'
import { Children, createContext, isValidElement, useContext, useState, type ComponentProps, type ReactNode } from 'react'
import { cn } from '@/lib/utils'
import { NativeSelect } from './native-select'
import { optionText } from './option-text'
import { defaultUiLabels } from './default-labels'
import { useUiLabels } from './labels-context'
import { Badge } from './badge'
import { tabsListVariants, tabsTriggerVariants, type TabsSize, type TabsVariant } from './tabs-variants'


const TabsStyleContext = createContext<{ size: TabsSize; variant: TabsVariant }>({ size: 'md', variant: 'line' })

const TabsValueContext = createContext<{ value: string; select: (value: string) => void } | null>(null)

export function Tabs({ value, defaultValue, onValueChange, ...props }: ComponentProps<typeof TabsPrimitive.Root>) {
  const [internal, setInternal] = useState(defaultValue)
  const select = (next: string) => {
    if (value === undefined) setInternal(next)
    onValueChange?.(next)
  }
  return <TabsValueContext.Provider value={{ value: value ?? internal ?? '', select }}>
    <TabsPrimitive.Root {...props} value={value ?? internal} onValueChange={select} />
  </TabsValueContext.Provider>
}

export function TabsList({ className, children, size = 'md', variant = 'line', ...props }: ComponentProps<typeof TabsPrimitive.List> & { size?: TabsSize; variant?: TabsVariant }) {
  const context = useContext(TabsValueContext)
  const labels = useUiLabels()
  const sectionLabel = labels.sectionNavigation ?? defaultUiLabels().sectionNavigation
  const options = Children.toArray(children).flatMap((child) => isValidElement<TabsTriggerProps>(child) && child.type === TabsTrigger ? [child.props] : [])
  return (
    <TabsStyleContext.Provider value={{ size, variant }}>
      {context && options.length > 0 && <NativeSelect className="w-full md:hidden" aria-label={props['aria-label'] ?? sectionLabel} value={context.value} onValueChange={context.select}>
        {!options.some((option) => option.value === context.value) && <option value="" disabled>{sectionLabel}</option>}
        {options.map((option) => <option key={option.value} value={option.value} disabled={option.disabled}>{option['aria-label'] ?? (optionText(option.children) || option.value)}{option.count !== undefined ? ` (${option.count})` : ''}</option>)}
      </NativeSelect>}
      <TabsPrimitive.List data-slot="tabs-list" className={cn(tabsListVariants({ size, variant }), className, context && options.length > 0 && 'hidden md:inline-flex')} {...props}>{children}</TabsPrimitive.List>
    </TabsStyleContext.Provider>
  )
}

export interface TabsTriggerProps extends ComponentProps<typeof TabsPrimitive.Trigger> {
  icon?: ReactNode
  count?: number
}

export function TabsTrigger({ className, icon, count, children, ...props }: TabsTriggerProps) {
  const { size, variant } = useContext(TabsStyleContext)
  return (
    <TabsPrimitive.Trigger data-slot="tabs-trigger" className={cn(tabsTriggerVariants({ size, variant }), className)} {...props}>
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


