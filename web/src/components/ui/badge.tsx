import { cva, type VariantProps } from 'class-variance-authority'
import type { ComponentProps, ReactNode } from 'react'
import { cn } from '@/lib/utils'

const badgeVariants = cva(
  'inline-flex w-fit shrink-0 items-center justify-center gap-1 whitespace-nowrap rounded-(--badge-radius) border font-medium [&>svg]:size-3 [&>svg]:shrink-0',
  {
    variants: {
      variant: {
        default: 'border-transparent bg-primary-solid text-primary-foreground',
        secondary: 'border-transparent bg-secondary text-secondary-foreground',
        destructive: 'border-transparent bg-destructive-solid text-destructive-foreground',
        success: 'border-transparent bg-success-solid text-success-foreground',
        warning: 'border-transparent bg-warning-solid text-warning-foreground',
        outline: 'text-foreground',
      },
      tone: {
        neutral: 'border-border bg-muted text-muted-foreground',
        primary: 'border-primary/20 bg-primary/10 text-primary',
        success: 'border-success/20 bg-success/10 text-success',
        warning: 'border-warning/20 bg-warning/10 text-warning',
        destructive: 'border-destructive/20 bg-destructive/10 text-destructive',
      },
      size: {
        sm: 'px-1.5 py-0 text-[0.6875rem] leading-4',
        md: 'px-2 py-0.5 text-xs',
      },
      count: {
        true: 'min-w-5 justify-center rounded-full px-1.5 py-0 tabular-nums',
        false: '',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'md',
      count: false,
    },
  },
)

export type BadgeTone = NonNullable<VariantProps<typeof badgeVariants>['tone']>

export interface BadgeProps extends Omit<ComponentProps<'span'>, 'children'>, Omit<VariantProps<typeof badgeVariants>, 'count'> {
  icon?: ReactNode
  count?: boolean
  children?: ReactNode
}

export function Badge({ className, variant, tone, size, count, icon, children, ...props }: BadgeProps) {
  const resolvedVariant = tone ? null : variant
  return (
    <span
      data-slot="badge"
      className={cn(badgeVariants({ variant: resolvedVariant, tone, size, count: count ?? false }), className)}
      {...props}
    >
      {icon}
      {children}
    </span>
  )
}
