import { useTranslation } from 'react-i18next'
import { NavLink, Outlet } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { cn } from '@/lib/utils'
import { CALLS_PATH } from './constants'

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
      <header className="grid gap-1">
        <h1 className="text-2xl font-semibold tracking-tight">{t('page.title')}</h1>
        <p className="text-sm text-muted-foreground">{t('page.description')}</p>
      </header>
      <nav aria-label={t('page.sections')} className="flex gap-1 overflow-x-auto border-b">
        {tabs.map((tab) => (
          <NavLink
            key={tab.to}
            to={tab.to}
            end={tab.end}
            className={({ isActive }) =>
              cn(
                '-mb-px border-b-2 px-3 py-2 text-sm font-medium whitespace-nowrap transition-colors',
                isActive
                  ? 'border-primary text-foreground'
                  : 'border-transparent text-muted-foreground hover:text-foreground',
              )
            }
          >
            {tab.label}
          </NavLink>
        ))}
      </nav>
      <Outlet />
    </div>
  )
}
