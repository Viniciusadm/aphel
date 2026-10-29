import { ChevronDown, ChevronRight, History, Plus, SquareCheck } from 'lucide-react'
import { useId, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { cn } from '@/lib/utils'
import { useRecordActivities, useToggleActivity, type Activity, type RecordTarget } from './api'
import { ActivityFormDialog, type ActivityFormMode } from './components/ActivityFormDialog'
import { ActivityRow } from './components/ActivityRow'
import { activityErrorKey } from './lib/errors'

export interface ActivitiesTabProps {
  recordType: string
  recordId: string
  readOnly?: boolean
  className?: string
}

export function ActivitiesTab({ recordType, recordId, readOnly = false, className }: ActivitiesTabProps) {
  const { t } = useTranslation('activities')
  const { hasPermission } = useSession()
  const titleId = useId()
  const target = useMemo<RecordTarget>(() => ({ recordType, recordId }), [recordType, recordId])
  const activities = useRecordActivities(target)
  const toggle = useToggleActivity(target)
  const [dialog, setDialog] = useState<ActivityFormMode | null>(null)
  const [showDone, setShowDone] = useState(false)
  const canCreate = !readOnly && hasPermission('activities.create')
  const canUpdate = !readOnly && hasPermission('activities.update')

  const items = useMemo(() => activities.data?.data ?? [], [activities.data])
  const open = items.filter((activity) => activity.status === 'open')
  const closed = items.filter((activity) => activity.status !== 'open')

  const onToggle = (activity: Activity, done: boolean) => {
    toggle.mutate(
      { id: activity.id, done },
      {
        onSuccess: () =>
          toast({
            title: t(done ? 'tab.completedToast' : 'tab.reopenedToast', { title: activity.title }),
            variant: 'success',
          }),
        onError: (error) => toast({ title: t(activityErrorKey(error)), variant: 'destructive' }),
      },
    )
  }
  const pendingId = toggle.isPending ? toggle.variables?.id : undefined

  return (
    <section aria-labelledby={titleId} className={cn('flex flex-col gap-4', className)}>
      <header className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <SquareCheck className="size-4 text-muted-foreground" aria-hidden="true" />
          <h2 id={titleId} className="text-base font-semibold">
            {t('tab.title')}
          </h2>
          {activities.isSuccess && (
            <span className="text-sm text-muted-foreground">{t('tab.openCount', { count: open.length })}</span>
          )}
        </div>
        {canCreate && (
          <div className="flex flex-wrap gap-2">
            <Button size="sm" variant="outline" onClick={() => setDialog('log')}>
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

      {activities.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-14 w-full" />
          <Skeleton className="h-14 w-full" />
        </div>
      ) : activities.isError ? (
        <ErrorState onRetry={() => void activities.refetch()} />
      ) : items.length === 0 ? (
        <EmptyState icon={SquareCheck} title={t('tab.emptyTitle')} description={t('tab.emptyDescription')} />
      ) : (
        <div className="grid gap-4">
          {open.length > 0 ? (
            <ul className="grid gap-2" aria-label={t('tab.openList')}>
              {open.map((activity) => (
                <ActivityRow
                  key={activity.id}
                  activity={activity}
                  onToggle={canUpdate ? onToggle : undefined}
                  pending={pendingId === activity.id}
                />
              ))}
            </ul>
          ) : (
            <p className="text-sm text-muted-foreground">{t('tab.noOpen')}</p>
          )}
          {closed.length > 0 && (
            <div className="grid gap-2">
              <Button
                variant="ghost"
                size="sm"
                className="w-fit"
                aria-expanded={showDone}
                onClick={() => setShowDone((current) => !current)}
              >
                {showDone ? <ChevronDown aria-hidden="true" /> : <ChevronRight aria-hidden="true" />}
                {t('tab.doneToggle', { count: closed.length })}
              </Button>
              {showDone && (
                <ul className="grid gap-2" aria-label={t('tab.doneList')}>
                  {closed.map((activity) => (
                    <ActivityRow
                      key={activity.id}
                      activity={activity}
                      onToggle={canUpdate ? onToggle : undefined}
                      pending={pendingId === activity.id}
                    />
                  ))}
                </ul>
              )}
            </div>
          )}
        </div>
      )}

      <ActivityFormDialog
        open={dialog !== null}
        mode={dialog ?? 'task'}
        target={target}
        onOpenChange={(value) => !value && setDialog(null)}
      />
    </section>
  )
}
