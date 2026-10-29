import type { ComponentProps } from 'react'
import { initials } from '@/lib/initials'
import { cn } from '@/lib/utils'

export interface AvatarProps extends ComponentProps<'span'> {
  name: string
  size?: 'sm' | 'md'
}

export function Avatar({ name, size = 'sm', className, ...props }: AvatarProps) {
  return (
    <span
      data-slot="avatar"
      aria-hidden="true"
      className={cn(
        'inline-flex shrink-0 items-center justify-center rounded-full bg-secondary font-medium text-secondary-foreground',
        size === 'sm' ? 'size-6 text-[0.625rem]' : 'size-8 text-xs',
        className,
      )}
      {...props}
    >
      {initials(name)}
    </span>
  )
}
