import { useCallback, useState } from 'react'
import type { AppFrameProps } from '@/kernel/modules/app-frame'
import { cn } from '@/lib/utils'
import { readStorage, writeStorage } from '@/lib/storage'
import { MobileSidebar } from './MobileSidebar'
import { Sidebar } from './Sidebar'
import { Topbar } from './Topbar'

const SIDEBAR_COLLAPSED_KEY = 'aphel.sidebar.collapsed'

export function DefaultAppFrame({ children, settingsArea, returnPath, mobileNavOpen, onMobileNavOpenChange }: AppFrameProps) {
  const [collapsed, setCollapsed] = useState(() => readStorage(SIDEBAR_COLLAPSED_KEY) === 'true')

  const toggleCollapsed = useCallback(() => {
    setCollapsed((current) => {
      const next = !current
      writeStorage(SIDEBAR_COLLAPSED_KEY, String(next))
      return next
    })
  }, [])

  return (
    <>
      <Sidebar
        collapsed={collapsed}
        onToggleCollapsed={toggleCollapsed}
        settingsArea={settingsArea}
        returnPath={returnPath}
      />
      <MobileSidebar
        open={mobileNavOpen}
        onOpenChange={onMobileNavOpenChange}
        settingsArea={settingsArea}
        returnPath={returnPath}
      />
      <div className="flex min-w-0 flex-1 flex-col">
        <Topbar />
        <main className={cn('flex-1 p-4 pb-[calc(5.5rem+env(safe-area-inset-bottom))] md:p-6', !settingsArea && 'overflow-y-auto')}>
          {children}
        </main>
      </div>
    </>
  )
}
