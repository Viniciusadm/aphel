import { useTranslation } from 'react-i18next'
import { Outlet } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { TabsNav, TabsNavItem } from '@/components/ui/tabs-nav'
import { CALLS_PATH } from './constants'
import { PageHeader } from '@/components/ui/page-header'

export function CallsLayout() {
  const { t } = useTranslation('calls')
  const { hasPermission } = useSession()
  const tabs = [
    { to: CALLS_PATH, label: t('page.sessions'), end: true },
    { to: `${CALLS_PATH}/report`, label: t('page.report'), end: false },
    ...(hasPermission('settings.manage')
      ? [{ to: `${CALLS_PATH}/outcomes`, label: t('page.outcomes'), end: false }]
      : []),
  ]
  return (
    <div className="flex w-full flex-col gap-4">
      <PageHeader title={t('page.title')} description={t('page.description')} />
      <TabsNav aria-label={t('page.sections')} className="self-start">
        {tabs.map((tab) => (
          <TabsNavItem key={tab.to} to={tab.to} end={tab.end}>
            {tab.label}
          </TabsNavItem>
        ))}
      </TabsNav>
      <Outlet />
    </div>
  )
}
