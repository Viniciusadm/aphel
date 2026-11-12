import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { toast } from '@/components/ui/toast-store'
import {
  activitiesApi,
  useInvalidateActivities,
  useRecordFormatting,
  useToggleActivity,
  type Activity,
  type UpdateActivityRequest,
} from '@/sdk'
import { postponedDueAt } from './dates'
import { activityErrorKey } from './errors'

const UNDO_MS = 8000

export function useActivityActions(activity: Activity) {
  const { t } = useTranslation('tasks')
  const { t: tl } = useTranslation('tasks', { keyPrefix: 'list' })
  const formatting = useRecordFormatting()
  const toggle = useToggleActivity()
  const invalidate = useInvalidateActivities()
  const [busy, setBusy] = useState(false)

  const fail = (error: unknown) =>
    toast({
      title: isApiError(error) && error.code === 'version_conflict' ? tl('errors.conflict') : t(activityErrorKey(error)),
      variant: 'destructive',
    })

  const undoAction = (run: () => Promise<unknown>) => ({
    label: tl('undo.action'),
    onClick: () => {
      void run().then(
        () => {
          toast({ title: tl('toast.undone'), variant: 'success' })
          return invalidate()
        },
        (error) => {
          fail(error)
          return invalidate()
        },
      )
    },
  })

  const setDone = (done: boolean) =>
    toggle.mutate(
      { id: activity.id, done },
      {
        onSuccess: () =>
          toast({
            title: t(done ? 'tab.completedToast' : 'tab.reopenedToast', { title: activity.title }),
            variant: 'success',
            duration: UNDO_MS,
            action: undoAction(() => (done ? activitiesApi.reopen(activity.id) : activitiesApi.complete(activity.id))),
          }),
        onError: fail,
      },
    )

  const change = async (request: UpdateActivityRequest, revert: UpdateActivityRequest, message: string) => {
    setBusy(true)
    try {
      const saved = await activitiesApi.update(activity.id, request, activity.updated_at)
      toast({
        title: message,
        variant: 'success',
        duration: UNDO_MS,
        action: undoAction(() => activitiesApi.update(saved.id, revert, saved.updated_at)),
      })
    } catch (error) {
      fail(error)
    } finally {
      setBusy(false)
      void invalidate()
    }
  }

  const postpone = (days: number) => {
    const dueAt = postponedDueAt(activity.due_at, days, formatting.timezone)
    if (!dueAt) return
    void change(
      { due_at: dueAt },
      { due_at: activity.due_at ?? null },
      tl(days === 1 ? 'toast.postponedDay' : 'toast.postponedWeek', { title: activity.title }),
    )
  }

  const reassign = (ownerId: string) =>
    change({ owner_id: ownerId }, { owner_id: activity.owner_id ?? null }, tl('toast.reassigned', { title: activity.title }))

  return {
    setDone,
    postpone,
    reassign,
    pending: busy || toggle.isPending,
  }
}
