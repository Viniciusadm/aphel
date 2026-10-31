import type { ComponentProps, ReactNode } from 'react'
import { cn } from '@/lib/utils'
import { Badge, type BadgeTone } from './badge'

export interface StatusBadgeProps extends Omit<ComponentProps<typeof Badge>, 'tone' | 'variant' | 'count' | 'icon'> {
  tone: BadgeTone
  icon?: ReactNode
  struck?: boolean
}

export function StatusBadge({ tone, icon, struck, className, children, ...props }: StatusBadgeProps) {
  return (
    <Badge data-slot="status-badge" tone={tone} icon={icon} className={cn(struck && 'line-through', className)} {...props}>
      {children}
    </Badge>
  )
}
