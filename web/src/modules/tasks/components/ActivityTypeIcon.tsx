import { createElement } from 'react'
import { cn } from '@/lib/utils'
import { DEFAULT_ACTIVITY_COLOR, activityIcon } from '../lib/icons'

export interface ActivityTypeIconProps {
  icon?: string | null
  color?: string | null
  className?: string
}

export function ActivityTypeIcon({ icon, color, className }: ActivityTypeIconProps) {
  const tint = color || DEFAULT_ACTIVITY_COLOR
  return (
    <span
      aria-hidden="true"
      className={cn('inline-flex size-7 shrink-0 items-center justify-center rounded-full', className)}
      style={{ color: tint, backgroundColor: `${tint}1f` }}
    >
      {createElement(activityIcon(icon), { className: 'size-4' })}
    </span>
  )
}
