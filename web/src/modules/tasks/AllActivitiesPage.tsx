import { CheckCheck } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { toast } from '@/components/ui/toast-store'
import { RecordListView } from '@/sdk'
import { activityResource } from './resource'
import { activitiesApi, useInvalidateActivities } from '@/sdk'
import { ACTIVITY_RECORD_TYPE } from '@/sdk'

export function AllActivitiesPage() {
  const { t } = useTranslation('tasks')
  const { hasPermission } = useSession()
  const navigate = useNavigate()
  const invalidate = useInvalidateActivities()

  const completeSelected = async (ids: string[], clearSelection: () => void) => {
    const results = await Promise.allSettled(ids.map((id) => activitiesApi.complete(id)))
    const failed = results.filter((result) => result.status === 'rejected').length
    await invalidate()
    clearSelection()
    if (failed > 0) {
      toast({ title: t('all.completeFailed', { count: failed }), variant: 'destructive' })
    } else {
      toast({ title: t('all.completed', { count: ids.length }), variant: 'success' })
    }
  }

  return (
    <RecordListView
      recordType={ACTIVITY_RECORD_TYPE}
      onRowClick={(record) => navigate(activityResource.detailPath?.(record.id) ?? '')}
      bulkActions={
        hasPermission('tasks.update')
          ? ({ selectedIds, clearSelection }) => (
              <Button size="sm" variant="outline" onClick={() => void completeSelected(selectedIds, clearSelection)}>
                <CheckCheck aria-hidden="true" />
                {t('all.completeSelected', { count: selectedIds.length })}
              </Button>
            )
          : undefined
      }
    />
  )
}
