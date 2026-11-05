import { PartyPopper } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { useRecordFormatting } from '@/sdk'
import {useMyOpenActivities, useToggleActivity, type Activity} from '@/sdk'
import { ActivityRow } from './components/ActivityRow'
import { groupByDue } from './lib/due'
import { activityErrorKey } from './lib/errors'

export interface MyTasksPageProps {
  now?: Date
}

export function MyTasksPage({ now }: MyTasksPageProps) {
  const { t } = useTranslation('tasks')
  const formatting = useRecordFormatting()
  const tasks = useMyOpenActivities()
  const toggle = useToggleActivity()

  if (tasks.isPending) {
    return (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-14 w-full" />
      </div>
    )
  }
  if (tasks.isError) return <ErrorState onRetry={() => void tasks.refetch()} />

  const items = tasks.data.pages.flatMap((page) => page.data)
  const total = tasks.data.pages.at(-1)?.meta.total ?? items.length
  if (items.length === 0) {
    return <EmptyState icon={PartyPopper} title={t('mine.emptyTitle')} description={t('mine.emptyDescription')} />
  }

  const onToggle = (activity: Activity, done: boolean) =>
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
  const pendingId = toggle.isPending ? toggle.variables?.id : undefined

  return (
    <div className="grid gap-6">
      {groupByDue(items, formatting.timezone, now).map((group) => (
        <section key={group.state} className="grid gap-2" aria-labelledby={`tasks-${group.state}`}>
          <h2 id={`tasks-${group.state}`} className="text-sm font-semibold text-muted-foreground">
            {t(`mine.groups.${group.state}`)} <span className="font-normal">({group.items.length})</span>
          </h2>
          <ul className="grid gap-2">
            {group.items.map((activity) => (
              <ActivityRow
                key={activity.id}
                activity={activity}
                onToggle={onToggle}
                pending={pendingId === activity.id}
                showRecord
                now={now}
              />
            ))}
          </ul>
        </section>
      ))}
      {tasks.hasNextPage && (
        <div className="flex flex-col items-center gap-2">
          <p className="text-sm text-muted-foreground">{t('mine.shown', { shown: items.length, total })}</p>
          <Button
            variant="outline"
            disabled={tasks.isFetchingNextPage}
            onClick={() => void tasks.fetchNextPage()}
          >
            {tasks.isFetchingNextPage ? t('mine.loadingMore') : t('mine.loadMore')}
          </Button>
        </div>
      )}
    </div>
  )
}
