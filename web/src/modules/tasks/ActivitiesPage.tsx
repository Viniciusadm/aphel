import { CheckCheck } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { toast } from '@/components/ui/toast-store'
import { RecordListView, type FilterNode, type RecordListPreset, type SortSpec } from '@/sdk'
import { activityResource } from './resource'
import { activitiesApi, useInvalidateActivities } from '@/sdk'
import { ACTIVITY_RECORD_TYPE } from '@/sdk'

const OPEN: FilterNode = { field: 'status', operator: 'eq', value: 'open' }
const MINE: FilterNode = { field: 'owner_id', operator: 'eq', value: 'me' }
const BY_DUE: SortSpec[] = [{ field: 'due_at', direction: 'asc' }]

export function ActivitiesPage() {
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

  const presets: RecordListPreset[] = [
    {
      key: 'mine',
      label: t('presets.mine'),
      filter: { op: 'and', conditions: [OPEN, MINE] },
      sort: BY_DUE,
      emptyTitle: t('mine.emptyTitle'),
      emptyDescription: t('mine.emptyDescription'),
    },
    { key: 'open', label: t('presets.open'), filter: OPEN, sort: BY_DUE },
    {
      key: 'done',
      label: t('presets.done'),
      filter: { field: 'status', operator: 'eq', value: 'done' },
      sort: [{ field: 'completed_at', direction: 'desc' }],
    },
    { key: 'all', label: t('presets.all'), filter: null },
  ]

  return (
    <RecordListView
      recordType={ACTIVITY_RECORD_TYPE}
      presets={presets}
      defaultPreset="mine"
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
