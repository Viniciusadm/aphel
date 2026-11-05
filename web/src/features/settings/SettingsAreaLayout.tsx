import { useMemo } from 'react'
import { Outlet, useMatch } from 'react-router'
import { useLabelTranslator } from '@/kernel/modules/i18n'
import { NoAccess } from './components/NoAccess'
import { useSettingsGroups, type SettingsArea } from '@/kernel/modules/settings'
import { useOtherAreas } from './lib/use-other-areas'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

interface SettingsAreaLayoutProps {
  area: SettingsArea
}

export function SettingsAreaLayout({ area }: SettingsAreaLayoutProps) {
  const translate = useLabelTranslator()
  const isOverview = useMatch(area.basePath) !== null
  const otherAreas = useOtherAreas(area)
  const [groups] = useSettingsGroups(useMemo(() => [area], [area]))
  const hasItems = groups.length > 0
  const hasContent = hasItems || (area.primary && otherAreas.length > 0)

  if (!isOverview && hasContent) return <Outlet />

  return (
    <Page>
      {(isOverview || !hasContent) && (
        <PageHeader title={translate('settings', area.titleKey)} description={translate('settings', area.descriptionKey)} />
      )}
      {hasContent ? <Outlet /> : <NoAccess />}
    </Page>
  )
}
