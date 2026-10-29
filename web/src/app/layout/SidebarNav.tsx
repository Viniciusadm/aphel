import { useTranslation } from 'react-i18next'
import { NavLink } from 'react-router'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { useNavigationPreferences } from '@/features/settings/navigation/api'
import { cn } from '@/lib/utils'
import { arrangeSidebarEntries, useAvailableSidebarEntries } from './sidebar-entries'

interface SidebarNavProps {
  collapsed?: boolean
  onNavigate?: () => void
}

export function SidebarNav({ collapsed = false, onNavigate }: SidebarNavProps) {
  const { t } = useTranslation('layout')
  const preferences = useNavigationPreferences()
  const entries = arrangeSidebarEntries(useAvailableSidebarEntries(), preferences)

  return (
    <nav aria-label={t('nav.label')} className="flex flex-col gap-1 px-2">
      {entries.map((entry) => {
        const link = (
          <NavLink
            key={entry.key}
            to={entry.to}
            end={entry.end}
            onClick={onNavigate}
            aria-label={collapsed ? entry.label : undefined}
            className={cn(
              'relative flex h-9 items-center gap-3 rounded-md px-3 text-sm font-medium text-sidebar-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none aria-[current=page]:bg-sidebar-accent aria-[current=page]:text-sidebar-accent-foreground',
              collapsed && 'justify-center px-0',
            )}
          >
            {entry.icon}
            {!collapsed && <span className="truncate">{entry.label}</span>}
            {entry.badge && <entry.badge collapsed={collapsed} />}
          </NavLink>
        )

        if (!collapsed) return link

        return (
          <Tooltip key={entry.key}>
            <TooltipTrigger asChild>{link}</TooltipTrigger>
            <TooltipContent side="right">{entry.label}</TooltipContent>
          </Tooltip>
        )
      })}
    </nav>
  )
}
