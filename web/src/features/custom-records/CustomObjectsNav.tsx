import { NavLink } from 'react-router'
import { NO_FILTER_CONTEXT, useFilters } from '@/kernel/modules/filters'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { cn } from '@/lib/utils'
import { useCustomObjects } from './api'
import { objectColor } from './icons'
import { ObjectIcon } from './ObjectIcon'
import { customObjectPath } from './resources'

export interface CustomObjectsNavProps {
  collapsed?: boolean
  onNavigate?: () => void
}

export function CustomObjectsNav({ collapsed = false, onNavigate }: CustomObjectsNavProps) {
  const objects = useCustomObjects()
  const visible = useFilters('navigation.customObjects', objects.data ?? [], NO_FILTER_CONTEXT)
  if (visible.length === 0) return null

  return (
    <>
      {visible.map((object) => {
        const label = object.label_plural
        const to = customObjectPath(object.key)
        const link = (
          <NavLink
            key={object.key}
            to={to}
            onClick={onNavigate}
            aria-label={collapsed ? label : undefined}
            className={cn(
              'flex h-9 items-center gap-3 rounded-md px-3 text-sm font-medium text-sidebar-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none aria-[current=page]:bg-sidebar-accent aria-[current=page]:text-sidebar-accent-foreground',
              collapsed && 'justify-center px-0',
            )}
          >
            <ObjectIcon
              icon={object.icon}
              className="size-4 shrink-0"
              style={{ color: objectColor(object.color) }}
              aria-hidden
            />
            {!collapsed && <span className="truncate">{label}</span>}
          </NavLink>
        )

        if (!collapsed) return link

        return (
          <Tooltip key={object.key}>
            <TooltipTrigger asChild>{link}</TooltipTrigger>
            <TooltipContent side="right">{label}</TooltipContent>
          </Tooltip>
        )
      })}
    </>
  )
}
