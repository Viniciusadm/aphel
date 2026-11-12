import { cva, type VariantProps } from 'class-variance-authority'

export const tabsListVariants = cva('inline-flex w-fit max-w-full items-center gap-1 max-md:h-auto max-md:flex-wrap rounded-md bg-muted text-muted-foreground', {
  variants: {
    size: {
      sm: 'h-(--control-height-sm) p-1',
      md: 'h-(--control-height-md) p-1.5',
    },
    variant: {
      pill: '',
      line: 'h-auto w-full gap-6 rounded-none border-b bg-transparent p-0 text-muted-foreground',
    },
  },
  defaultVariants: { size: 'md', variant: 'line' },
})

export const tabsTriggerVariants = cva(
  'inline-flex h-full max-md:min-h-11 shrink-0 items-center justify-center gap-1.5 whitespace-nowrap rounded-sm font-medium outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 aria-pressed:bg-background aria-pressed:text-foreground aria-pressed:shadow-sm aria-[current=page]:bg-background aria-[current=page]:text-foreground aria-[current=page]:shadow-sm data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm data-[state=on]:bg-background data-[state=on]:text-foreground data-[state=on]:shadow-sm [&>svg]:size-4 [&>svg]:shrink-0',
  {
    variants: {
      size: {
        sm: 'px-2.5 text-xs',
        md: 'px-3 text-sm',
      },
      variant: {
        pill: '',
        line: 'relative -mb-px h-auto rounded-none outline-none focus-visible:text-foreground focus-visible:ring-0 border-b-[3px] border-transparent px-1 pb-3 pt-2 text-base font-medium text-foreground/70 hover:border-border data-[state=active]:border-primary-solid data-[state=active]:bg-transparent data-[state=active]:font-semibold data-[state=active]:text-foreground data-[state=active]:shadow-none aria-[current=page]:border-primary-solid aria-[current=page]:bg-transparent aria-[current=page]:font-semibold aria-[current=page]:text-foreground aria-[current=page]:shadow-none data-[state=on]:border-primary-solid data-[state=on]:bg-transparent data-[state=on]:text-foreground data-[state=on]:shadow-none',
      },
    },
    defaultVariants: { size: 'md', variant: 'line' },
  },
)

export type TabsSize = NonNullable<VariantProps<typeof tabsListVariants>['size']>
export type TabsVariant = NonNullable<VariantProps<typeof tabsListVariants>['variant']>
