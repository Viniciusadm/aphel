import { useTranslation } from 'react-i18next'
import { Navigate, NavLink, Outlet } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { EmptyState } from '@/components/ui/empty-state'
import { cn } from '@/lib/utils'
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
          <nav aria-label={t('tabs.label')} className="flex gap-1 overflow-x-auto shadow-[inset_0_-1px_0_var(--color-border)]">
            {tabs.map((tab) => (
              <NavLink
                key={tab.path}
                to={tab.path}
                className={({ isActive }) =>
                  cn(
                    'border-b-2 px-3 py-2 text-sm font-medium whitespace-nowrap transition-colors',
                    isActive
                      ? 'border-primary text-foreground'
                      : 'border-transparent text-muted-foreground hover:text-foreground',
                  )
                }
              >
                {t(tab.labelKey)}
              </NavLink>
            ))}
          </nav>
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
