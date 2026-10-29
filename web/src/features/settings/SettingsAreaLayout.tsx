import { useMemo } from 'react'
import { Outlet, useMatch } from 'react-router'
import { useTranslation } from 'react-i18next'
import { NoAccess } from './components/NoAccess'
import { useSettingsGroups, type SettingsArea } from '@/kernel/modules/settings'
import { useOtherAreas } from './lib/use-other-areas'

interface SettingsAreaLayoutProps {
  area: SettingsArea
}

export function SettingsAreaLayout({ area }: SettingsAreaLayoutProps) {
  const { t } = useTranslation('settings')
  const isOverview = useMatch(area.basePath) !== null
  const otherAreas = useOtherAreas(area)
  const [groups] = useSettingsGroups(useMemo(() => [area], [area]))
  const hasItems = groups.length > 0
  const hasContent = hasItems || (area.key === 'organization' && otherAreas.length > 0)

  return (
    <div className="flex w-full flex-col gap-6">
      {isOverview && (
        <header className="grid gap-1">
          <h1 className="text-2xl font-semibold tracking-tight">{t(area.titleKey)}</h1>
          <p className="text-sm text-muted-foreground">{t(area.descriptionKey)}</p>
        </header>
      )}
      {hasContent ? <Outlet /> : <NoAccess />}
    </div>
  )
}
