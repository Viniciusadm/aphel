import { useMemo } from 'react'
import { Outlet, useMatch } from 'react-router'
import { useTranslation } from 'react-i18next'
import { NoAccess } from './components/NoAccess'
import { useSettingsGroups, type SettingsArea } from '@/kernel/modules/settings'
import { useOtherAreas } from './lib/use-other-areas'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

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

  if (!isOverview && hasContent) return <Outlet />

  return (
    <Page>
      {(isOverview || !hasContent) && (
        <PageHeader title={t(area.titleKey)} description={t(area.descriptionKey)} />
      )}
      {hasContent ? <Outlet /> : <NoAccess />}
    </Page>
  )
}
