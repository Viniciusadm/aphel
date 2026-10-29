import { PanelLeftClose, PanelLeftOpen } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { Slot } from '@/kernel/modules/Slot'
import { SettingsNav } from '@/features/settings/components/SettingsNav'
import type { SettingsArea } from '@/kernel/modules/settings'
import { OrganizationBrand } from '@/features/settings/organization/OrganizationBrand'
import { SidebarNav } from './SidebarNav'

interface SidebarProps {
  collapsed: boolean
  onToggleCollapsed: () => void
  settingsArea: SettingsArea | null
  returnPath: string
}

export function Sidebar({ collapsed, onToggleCollapsed, settingsArea, returnPath }: SidebarProps) {
  const { t } = useTranslation('layout')
  const toggleLabel = collapsed ? t('sidebar.expand') : t('sidebar.collapse')
  const ToggleIcon = collapsed ? PanelLeftOpen : PanelLeftClose

  return (
    <aside
      data-collapsed={collapsed}
      className={cn(
        'hidden shrink-0 flex-col border-r border-sidebar-border bg-sidebar transition-[width] duration-200 md:flex',
        settingsArea && 'sticky top-0 h-dvh self-start',
        collapsed ? 'w-16' : 'w-64',
      )}
    >
      <div className={cn('flex h-14 items-center px-4', collapsed && 'justify-center px-2')}>
        <OrganizationBrand compact={collapsed} />
      </div>
      <div className="scroll-smooth-thin min-h-0 flex-1 overflow-y-auto py-2">
        {settingsArea ? (
          <SettingsNav area={settingsArea} returnPath={returnPath} collapsed={collapsed} />
        ) : (
          <>
            <SidebarNav collapsed={collapsed} />
            <Slot name="app.sidebar" props={{ collapsed }} />
          </>
        )}
      </div>
      <div className={cn('flex border-t border-sidebar-border p-2', collapsed ? 'justify-center' : 'justify-end')}>
        <Button
          variant="ghost"
          size="icon"
          onClick={onToggleCollapsed}
          aria-label={toggleLabel}
          aria-expanded={!collapsed}
        >
          <ToggleIcon />
        </Button>
      </div>
    </aside>
  )
}
