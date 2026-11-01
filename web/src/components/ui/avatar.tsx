import type { ComponentProps } from 'react'
import { initials } from '@/lib/initials'
import { cn } from '@/lib/utils'

export interface AvatarProps extends ComponentProps<'span'> {
  name: string
  size?: 'sm' | 'md' | 'lg' | 'xl'
}

const sizeClasses = {
  sm: 'size-6 text-[0.625rem]',
  md: 'size-8 text-xs',
  lg: 'size-12 text-base',
  xl: 'size-16 text-xl',
}

export function Avatar({ name, size = 'sm', className, ...props }: AvatarProps) {
  return (
    <span
      data-slot="avatar"
      aria-hidden="true"
      className={cn(
        'inline-flex shrink-0 items-center justify-center rounded-full bg-secondary font-medium text-secondary-foreground',
        sizeClasses[size],
        className,
      )}
      {...props}
    >
      {initials(name)}
    </span>
  )
}
