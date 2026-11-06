import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Outlet, useLocation } from 'react-router'
import { ErrorBoundary } from '@/components/feedback/ErrorBoundary'
import { PageLoader } from '@/components/feedback/PageLoader'
import { CommandPaletteProvider, useCommandPalette } from '@/features/command-palette'
import { MobileShell } from '@/features/mobile/MobileShell'
import { useSettingsArea } from '@/features/settings/lib/area-handle'
import { OrganizationTheme } from '@/features/settings/organization/OrganizationTheme'
import { TwoFactorGate } from '@/features/settings/security/two-factor/TwoFactorGate'
import { useNavigationPreferences } from '@/features/settings/navigation/api'
import { APP_FRAME_CONTRACT, type AppFrameProps } from '@/kernel/modules/app-frame'
import { UiPluginProvider } from '@/kernel/modules/ui-plugin'
import { useSurface, useUiPlugin } from '@/kernel/modules/ui-plugin-context'
import { useNavigationEntries } from '@/kernel/modules/navigation'
import { Slot } from '@/kernel/modules/Slot'
import { cn } from '@/lib/utils'
import { OfflineShell } from '@/pwa/offline/OfflineShell'
import { PwaShell } from '@/pwa/PwaShell'
import { BlueprintNotice } from './BlueprintNotice'
import { DefaultAppFrame } from './DefaultAppFrame'
import { ExtensionLoadNotice } from './ExtensionLoadNotice'
import { HomeRedirect } from './HomeRedirect'
import { applyStoredLanguage } from './language'
import { LanguageSync } from './LanguageSync'
import { arrangeSidebarEntries } from './sidebar-entries'

applyStoredLanguage()

function AppFrameContent() {
  const { t } = useTranslation('layout')
  const { ready } = useUiPlugin()
  const surface = useSurface<AppFrameProps>(APP_FRAME_CONTRACT)
  const Frame = surface.Component ?? DefaultAppFrame
  const navigation = arrangeSidebarEntries(useNavigationEntries(), useNavigationPreferences())
  const commands = useCommandPalette()
  const settingsArea = useSettingsArea()
  const location = useLocation()
  const currentPath = `${location.pathname}${location.search}${location.hash}`
  const [returnPath, setReturnPath] = useState('/')
  if (!settingsArea && returnPath !== currentPath) setReturnPath(currentPath)
  const [mobileNavOpen, setMobileNavOpen] = useState(false)

  const frameProps = { navigation, commands, settingsArea, returnPath, mobileNavOpen, onMobileNavOpenChange: setMobileNavOpen }
  const content = ready ? <><BlueprintNotice /><Outlet /></> : <PageLoader label={t('ui.loading')} />

  return (
    <MobileShell onOpenMenu={() => setMobileNavOpen(true)}>
      <div className={cn('flex min-h-dvh bg-background text-foreground', !settingsArea && 'h-full')}>
        <OrganizationTheme />
        <Slot name="app.shell" props={{}} />
        <TwoFactorGate />
        <LanguageSync />
        <ExtensionLoadNotice />
        <HomeRedirect />
        <ErrorBoundary resetKeys={[Frame, surface.plugin]} fallback={() => (
          <DefaultAppFrame {...frameProps}>
            {content}
          </DefaultAppFrame>
        )}>
          <Frame {...frameProps}>
            {content}
          </Frame>
        </ErrorBoundary>
        <PwaShell />
        <OfflineShell />
      </div>
    </MobileShell>
  )
}

export function AppLayout() {
  return (
    <UiPluginProvider>
      <CommandPaletteProvider>
        <AppFrameContent />
      </CommandPaletteProvider>
    </UiPluginProvider>
  )
}
