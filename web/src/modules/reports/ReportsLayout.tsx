import { useTranslation } from 'react-i18next'
import { Navigate, Outlet } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { EmptyState } from '@/components/ui/empty-state'
import { TabsNav, TabsNavItem } from '@/components/ui/tabs-nav'
import { allowedReportTabs } from './tabs'

export function ReportsLayout() {
  const { t } = useTranslation('reports')
  const { hasPermission } = useSession()
  const tabs = allowedReportTabs(hasPermission)

  return (
    <div className="flex w-full flex-col gap-6">
      <header className="grid gap-1">
        <h1 className="text-2xl font-semibold tracking-tight">{t('title')}</h1>
        <p className="text-sm text-muted-foreground">{t('description')}</p>
      </header>
      {tabs.length === 0 ? (
        <EmptyState title={t('noAccess.title')} description={t('noAccess.description')} />
      ) : (
        <>
          <TabsNav aria-label={t('tabs.label')} className="self-start">
            {tabs.map((tab) => (
              <TabsNavItem key={tab.path} to={tab.path}>
                {t(tab.labelKey)}
              </TabsNavItem>
            ))}
          </TabsNav>
          <Outlet />
        </>
      )}
    </div>
  )
}

export function ReportsIndex() {
  const { hasPermission } = useSession()
  const [first] = allowedReportTabs(hasPermission)
  return first ? <Navigate to={first.path} replace /> : null
}
