import { SidebarLink } from '@/components/ui/sidebar-link'
import { ArrowLeft, type LucideIcon } from 'lucide-react'
import type { ReactElement } from 'react'
import { Link, NavLink, useLocation } from 'react-router'
import { useTranslation } from 'react-i18next'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { cn } from '@/lib/utils'
import type { SettingsArea } from '@/kernel/modules/settings'
import { useOtherAreas } from '../lib/use-other-areas'
import { useSettingsSearch } from '../lib/use-settings-search'
import { SettingsSearchInput } from './SettingsSearchInput'

const headingClass = 'px-3 text-xs font-semibold tracking-wide text-muted-foreground uppercase'

interface SettingsNavProps {
  area: SettingsArea
  returnPath: string
  collapsed?: boolean
  onNavigate?: () => void
}

export function SettingsNav({ area, returnPath, collapsed = false, onNavigate }: SettingsNavProps) {
  const { t } = useTranslation('settings')
  const { t: tl } = useTranslation('layout')
  const location = useLocation()
  const { query, setQuery, filteredGroups, isSearching } = useSettingsSearch(area)
  const otherAreas = useOtherAreas(area)

  const withTooltip = (key: string, label: string, element: ReactElement) =>
    collapsed ? (
      <Tooltip key={key}>
        <TooltipTrigger asChild>{element}</TooltipTrigger>
        <TooltipContent side="right">{label}</TooltipContent>
      </Tooltip>
    ) : (
      element
    )

  const renderLink = (to: string, label: string, Icon: LucideIcon, end = false) =>
    withTooltip(
      to,
      label,
      <SidebarLink asChild key={to} collapsed={collapsed}>
        <NavLink
          key={to}
          to={to}
          end={end}
          onClick={onNavigate}
          aria-label={collapsed ? label : undefined}
        >
          <Icon aria-hidden className="size-4 shrink-0" />
          {!collapsed && <span className="truncate">{label}</span>}
        </NavLink>
      </SidebarLink>,
    )

  const areaPath = area.basePath.replace(/\/$/, '')
  const isAreaRoot = location.pathname.replace(/\/$/, '') === areaPath
  const backPath = isAreaRoot ? returnPath : area.basePath
  const backLabel = isAreaRoot ? tl('sidebar.backToApp') : t(area.titleKey)

  return (
    <div className="flex flex-col gap-4 px-2">
      <div className="grid gap-2">
        {withTooltip(
          'back',
          backLabel,
          <SidebarLink asChild collapsed={collapsed} className="text-muted-foreground">
            <Link
              to={backPath}
              onClick={onNavigate}
              aria-label={collapsed ? backLabel : undefined}
            >
              <ArrowLeft aria-hidden className="size-4 shrink-0" />
              {!collapsed && <span className="truncate">{backLabel}</span>}
            </Link>
          </SidebarLink>,
        )}
        {!collapsed && <p className="truncate px-3 text-sm font-semibold">{t(area.titleKey)}</p>}
        {!collapsed && <SettingsSearchInput value={query} onChange={setQuery} />}
      </div>
      <nav aria-label={t(area.titleKey)} className={cn('flex flex-col', collapsed ? 'gap-3' : 'gap-5')}>
        {filteredGroups.length === 0 && <p className="px-3 text-sm text-muted-foreground">{t('search.empty')}</p>}
        {filteredGroups.map((group) => (
          <div key={group.key} className="grid gap-1">
            {!collapsed && <h2 className={headingClass}>{group.label}</h2>}
            {group.items.map((item) => renderLink(item.to, item.label, item.icon))}
          </div>
        ))}
        {!isSearching && otherAreas.length > 0 && (
          <div className="grid gap-1 border-t border-sidebar-border pt-4">
            {!collapsed && <h2 className={headingClass}>{t('otherAreas')}</h2>}
            {otherAreas.map((other) => renderLink(other.basePath, t(other.titleKey), other.icon, true))}
          </div>
        )}
      </nav>
    </div>
  )
}
