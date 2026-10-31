import { cva, type VariantProps } from 'class-variance-authority'

export const tabsListVariants = cva('inline-flex max-w-full items-center gap-1 overflow-x-auto rounded-md bg-muted p-1 text-muted-foreground', {
  variants: {
    size: {
      sm: 'h-(--control-height-sm)',
      md: 'h-(--control-height-md)',
    },
  },
  defaultVariants: { size: 'md' },
})

export const tabsTriggerVariants = cva(
  'inline-flex h-full shrink-0 items-center justify-center gap-1.5 whitespace-nowrap rounded-sm font-medium outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 aria-pressed:bg-background aria-pressed:text-foreground aria-pressed:shadow-sm aria-[current=page]:bg-background aria-[current=page]:text-foreground aria-[current=page]:shadow-sm data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm data-[state=on]:bg-background data-[state=on]:text-foreground data-[state=on]:shadow-sm [&>svg]:size-4 [&>svg]:shrink-0',
  {
    variants: {
      size: {
        sm: 'px-2.5 text-xs',
        md: 'px-3 text-sm',
      },
    },
    defaultVariants: { size: 'md' },
  },
)

export type TabsSize = NonNullable<VariantProps<typeof tabsListVariants>['size']>
