import { History, Plus } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Outlet, useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { ActivityFormDialog, type ActivityFormMode } from './components/ActivityFormDialog'
import { TASKS_PATH } from '@/sdk'
import { activityResource } from './resource'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function TasksLayout() {
  const { t } = useTranslation('tasks')
  const { hasPermission } = useSession()
  const navigate = useNavigate()
  const [dialog, setDialog] = useState<ActivityFormMode | null>(null)
  return (
    <Page>
<PageHeader title={t('page.title')} description={t('page.description')} actions={<>
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
