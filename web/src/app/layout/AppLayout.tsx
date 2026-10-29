import { useCallback, useState } from 'react'
import { Outlet, useLocation } from 'react-router'
import { CommandPaletteProvider } from '@/features/command-palette'
import { MobileShell } from '@/features/mobile/MobileShell'
import { useSettingsArea } from '@/features/settings/lib/area-handle'
import { OrganizationTheme } from '@/features/settings/organization/OrganizationTheme'
import { TwoFactorGate } from '@/features/settings/security/two-factor/TwoFactorGate'
import { Slot } from '@/kernel/modules/Slot'
import { readStorage, writeStorage } from '@/lib/storage'
import { OfflineShell } from '@/pwa/offline/OfflineShell'
import { PwaShell } from '@/pwa/PwaShell'
import { HomeRedirect } from './HomeRedirect'
import { applyStoredLanguage } from './language'
import { LanguageSync } from './LanguageSync'
import { MobileSidebar } from './MobileSidebar'
import { Sidebar } from './Sidebar'
import { Topbar } from './Topbar'

const SIDEBAR_COLLAPSED_KEY = 'aphel.sidebar.collapsed'

applyStoredLanguage()

export function AppLayout() {
  const [collapsed, setCollapsed] = useState(() => readStorage(SIDEBAR_COLLAPSED_KEY) === 'true')
  const [mobileNavOpen, setMobileNavOpen] = useState(false)
  const settingsArea = useSettingsArea()
  const location = useLocation()
  const currentPath = `${location.pathname}${location.search}`
  const [returnPath, setReturnPath] = useState('/')
  if (!settingsArea && returnPath !== currentPath) setReturnPath(currentPath)

  const toggleCollapsed = useCallback(() => {
    setCollapsed((current) => {
      const next = !current
      writeStorage(SIDEBAR_COLLAPSED_KEY, String(next))
      return next
    })
  }, [])

  return (
    <CommandPaletteProvider>
      <div className="flex h-full min-h-screen bg-background text-foreground">
        <OrganizationTheme />
        <Slot name="app.shell" props={{}} />
        <TwoFactorGate />
        <LanguageSync />
        <HomeRedirect />
        <Sidebar
          collapsed={collapsed}
          onToggleCollapsed={toggleCollapsed}
          settingsArea={settingsArea}
          returnPath={returnPath}
        />
        <MobileSidebar
          open={mobileNavOpen}
          onOpenChange={setMobileNavOpen}
          settingsArea={settingsArea}
          returnPath={returnPath}
        />
        <div className="flex min-w-0 flex-1 flex-col">
          <Topbar onOpenMobileNav={() => setMobileNavOpen(true)} />
          <main className="flex-1 overflow-y-auto p-4 pb-[calc(5.5rem+env(safe-area-inset-bottom))] md:p-6">
            <Outlet />
          </main>
        </div>
        <PwaShell />
        <OfflineShell />
        <MobileShell onOpenMenu={() => setMobileNavOpen(true)} />
      </div>
    </CommandPaletteProvider>
  )
}
