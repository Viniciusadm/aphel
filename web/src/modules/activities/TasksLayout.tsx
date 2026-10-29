import { History, Plus } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { NavLink, Outlet, useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { ActivityFormDialog, type ActivityFormMode } from './components/ActivityFormDialog'
import { activityResource, TASKS_PATH } from './resource'

export function TasksLayout() {
  const { t } = useTranslation('activities')
  const { hasPermission } = useSession()
  const navigate = useNavigate()
  const [dialog, setDialog] = useState<ActivityFormMode | null>(null)
  const tabs = [
    { to: TASKS_PATH, label: t('page.mine'), end: true },
    { to: `${TASKS_PATH}/all`, label: t('page.all'), end: false },
  ]

  return (
    <div className="flex w-full flex-col gap-4">
      <header className="flex flex-wrap items-start justify-between gap-3">
        <div className="grid gap-1">
          <h1 className="text-2xl font-semibold tracking-tight">{t('page.title')}</h1>
          <p className="text-sm text-muted-foreground">{t('page.description')}</p>
        </div>
        {hasPermission('activities.create') && (
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" size="sm" onClick={() => setDialog('log')}>
              <History aria-hidden="true" />
              {t('tab.log')}
            </Button>
            <Button size="sm" onClick={() => setDialog('task')}>
              <Plus aria-hidden="true" />
              {t('tab.newTask')}
            </Button>
          </div>
        )}
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
      <ActivityFormDialog
        open={dialog !== null}
        mode={dialog ?? 'task'}
        onOpenChange={(value) => !value && setDialog(null)}
        onCreated={(activity) => {
          if (dialog === 'log') navigate(activityResource.detailPath?.(activity.id) ?? TASKS_PATH)
        }}
      />
    </div>
  )
}
