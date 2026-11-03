import { History, Plus } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Outlet, useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { AreaNavigation } from '@/components/ui/area-navigation'
import { useLocation } from 'react-router'
import { ActivityFormDialog, type ActivityFormMode } from './components/ActivityFormDialog'
import { activityResource, TASKS_PATH } from './resource'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function TasksLayout() {
  const { t } = useTranslation('tasks')
  const location = useLocation()
  const { hasPermission } = useSession()
  const navigate = useNavigate()
  const [dialog, setDialog] = useState<ActivityFormMode | null>(null)
  const tabs = [
    { to: TASKS_PATH, label: t('page.mine'), end: true },
    { to: `${TASKS_PATH}/all`, label: t('page.all'), end: false },
  ]

  return (
    <Page>
<PageHeader title={t('page.title')} actions={<>
        {hasPermission('tasks.create') && (
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
      
      </>} />
      <AreaNavigation items={tabs} label={t('page.sections')} active={tabs.find((item) => item.to === location.pathname)?.to ?? tabs[0].to} />
      <Outlet />
      <ActivityFormDialog
        open={dialog !== null}
        mode={dialog ?? 'task'}
        onOpenChange={(value) => !value && setDialog(null)}
        onCreated={(activity) => {
          if (dialog === 'log') navigate(activityResource.detailPath?.(activity.id) ?? TASKS_PATH)
        }}
      />
    </Page>
  )
}
