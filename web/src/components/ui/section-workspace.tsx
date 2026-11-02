import { Children, createContext, isValidElement, useContext, useId, useState, type ComponentProps, type ReactNode } from 'react'
import { useSearchParams } from 'react-router'
import { ChevronDown } from 'lucide-react'
import { useUiLabels } from './labels-context'
import { defaultUiLabels } from './default-labels'
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

export function SectionIndex({ className, children, ...props }: ComponentProps<'nav'>) {
  const context = useSection()
  const [open, setOpen] = useState(false)
  const labels = useUiLabels()
  const selected = Children.toArray(children).find((child) => isValidElement<{ value?: string }>(child) && child.props.value === context.active)
  const label = isValidElement<{ children?: ReactNode }>(selected) ? selected.props.children : (props['aria-label'] ?? labels.sectionNavigation ?? defaultUiLabels().sectionNavigation)
  return <nav {...props} className={cn('min-w-0 lg:sticky lg:top-20 lg:border-r lg:pr-4', className)}>
    <details open={open} onToggle={(event) => setOpen(event.currentTarget.open)} className="rounded-md border lg:hidden">
      <summary className="flex min-h-11 cursor-pointer list-none items-center justify-between gap-2 px-3 text-sm font-medium">{label}<ChevronDown className="size-4" aria-hidden /></summary>
      <div className="grid gap-1 border-t p-1" onClick={(event) => { if ((event.target as HTMLElement).closest('button')) setOpen(false) }}>{children}</div>
    </details>
    <div className="hidden gap-1 lg:grid">{children}</div>
  </nav>
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
