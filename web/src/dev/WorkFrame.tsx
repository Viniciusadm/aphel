import { NavLink } from 'react-router'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { SettingsNav } from '@/features/settings/components/SettingsNav'
import type { AppFrameProps } from '@/kernel/modules/app-frame'
import { cn } from '@/lib/utils'
import { MobileSidebar } from '@/app/layout/MobileSidebar'
import { Topbar } from '@/app/layout/Topbar'

export function WorkFrame({ children, navigation, commands, settingsArea, returnPath, mobileNavOpen, onMobileNavOpenChange }: AppFrameProps) {
  const { t } = useTranslation('layout')

  return (
    <div className="flex min-w-0 flex-1 flex-col">
      <Topbar />
      <nav aria-label={t('nav.label')} className="flex shrink-0 items-center gap-2 overflow-x-auto border-b p-2">
        {navigation.map((entry) => (
          <NavLink key={entry.key} to={entry.to} end={entry.end} className={({ isActive }) => cn('shrink-0 rounded-md px-3 py-2 text-sm hover:bg-accent', isActive && 'bg-accent text-accent-foreground')}>
            {entry.label}
          </NavLink>
        ))}
        <Button variant="ghost" onClick={() => commands.openPalette()}>{t('search.label')}</Button>
      </nav>
      <MobileSidebar open={mobileNavOpen} onOpenChange={onMobileNavOpenChange} settingsArea={settingsArea} returnPath={returnPath} />
      {settingsArea && <SettingsNav area={settingsArea} returnPath={returnPath} />}
      <main className={cn('flex-1 p-4 pb-[calc(5.5rem+env(safe-area-inset-bottom))] md:p-6', !settingsArea && 'overflow-y-auto')}>
        {children}
      </main>
    </div>
  )
}

