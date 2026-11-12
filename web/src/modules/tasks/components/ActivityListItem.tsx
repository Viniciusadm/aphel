import { useTranslation } from 'react-i18next'
import { toast } from '@/components/ui/toast-store'
import { useToggleActivity, type Activity } from '@/sdk'
import { activityErrorKey } from '../lib/errors'
import { ActivityRow } from './ActivityRow'

export function ActivityListItem({ record }: { record: unknown }) {
  const { t } = useTranslation('tasks')
  const activity = record as Activity
  const toggle = useToggleActivity()

  const onToggle = (item: Activity, done: boolean) =>
    toggle.mutate(
      { id: item.id, done },
      {
        onSuccess: () =>
          toast({
            title: t(done ? 'tab.completedToast' : 'tab.reopenedToast', { title: item.title }),
            variant: 'success',
          }),
        onError: (error) => toast({ title: t(activityErrorKey(error)), variant: 'destructive' }),
      },
    )

  return (
    <ActivityRow
      activity={activity}
      onToggle={onToggle}
      pending={toggle.isPending && toggle.variables?.id === activity.id}
      showRecord
    />
  )
}
