import { Children, createContext, isValidElement, useContext, useId, type ComponentProps } from 'react'
import { useSearchParams } from 'react-router'
import { cn } from '@/lib/utils'

interface SectionContextValue {
  active: string
  select: (value: string) => void
  id: string
}

const SectionContext = createContext<SectionContextValue | null>(null)

function useSection() {
  const context = useContext(SectionContext)
  if (!context) throw new Error('Section components require SectionWorkspace')
  return context
}

export function SectionWorkspace({ value, defaultValue = '', onValueChange, className, children, ...props }: Omit<ComponentProps<'div'>, 'onChange'> & {
  value?: string
  defaultValue?: string
  onValueChange?: (value: string) => void
}) {
  const [params, setParams] = useSearchParams()
  const id = useId()
  const keys = Children.toArray(children).flatMap((child) => isValidElement<{ value?: string }>(child) && child.type === SectionContent && child.props.value ? [child.props.value] : [])
  const requested = value ?? params.get('tab') ?? defaultValue
  const active = keys.length && !keys.includes(requested) ? (keys.includes(defaultValue) ? defaultValue : keys[0]) : requested
  const select = (next: string) => {
    if (onValueChange) onValueChange(next)
    if (value === undefined) setParams((current) => {
      const result = new URLSearchParams(current)
      result.set('tab', next)
      return result
    })
  }
  return (
    <SectionContext.Provider value={{ active, select, id }}>
      <div {...props} className={cn('grid min-w-0 items-start gap-6 lg:grid-cols-[12rem_minmax(0,1fr)]', className)}>{children}</div>
    </SectionContext.Provider>
  )
}

export function SectionIndex({ className, ...props }: ComponentProps<'nav'>) {
  return <nav {...props} className={cn('grid gap-1 border-b pb-4 lg:sticky lg:top-20 lg:border-r lg:border-b-0 lg:pr-4 lg:pb-0', className)} />
}

export function SectionLink({ value, children, className, ...props }: Omit<ComponentProps<'button'>, 'value'> & { value: string }) {
  const context = useSection()
  return <button {...props} type="button" aria-current={context.active === value ? 'page' : undefined}
    onClick={() => context.select(value)} aria-controls={`${context.id}-${value}`}
    className={cn('flex min-h-11 w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm whitespace-normal focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none', context.active === value ? 'bg-accent font-semibold text-accent-foreground' : 'text-muted-foreground hover:bg-muted hover:text-foreground', className)}>{children}</button>
}

export function SectionContent({ value, forceMount, className, ...props }: ComponentProps<'section'> & { value: string; forceMount?: boolean }) {
  const context = useSection()
  const active = context.active === value
  if (!active && !forceMount) return null
  return <section {...props} id={`${context.id}-${value}`} hidden={!active} data-state={active ? 'active' : 'inactive'} className={cn('min-w-0 lg:col-start-2 lg:row-start-1', className)} />
}
