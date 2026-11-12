import { CalendarPlus, Check, MoreHorizontal, UserRoundPen } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { toast } from '@/components/ui/toast-store'
import { activitiesApi, useInvalidateActivities, useRecordFormatting, useToggleActivity, type Activity, type UpdateActivityRequest } from '@/sdk'
import { postponedDueAt } from '../lib/dates'
import { activityErrorKey } from '../lib/errors'
import { ActivityRow } from './ActivityRow'
import { ReassignDialog, type ReassignTarget } from './ReassignDialog'

const UNDO_MS = 8000

export function ActivityListItem({ record }: { record: unknown }) {
  const { t } = useTranslation('tasks')
  const { t: tl } = useTranslation('tasks', { keyPrefix: 'list' })
  const activity = record as Activity
  const formatting = useRecordFormatting()
  const toggle = useToggleActivity()
  const invalidate = useInvalidateActivities()
  const [busy, setBusy] = useState(false)
  const [reassigning, setReassigning] = useState<ReassignTarget | null>(null)
  const open = activity.status === 'open'

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

  const onToggle = (item: Activity, done: boolean) =>
    toggle.mutate(
      { id: item.id, done },
      {
        onSuccess: () =>
          toast({
            title: t(done ? 'tab.completedToast' : 'tab.reopenedToast', { title: item.title }),
            variant: 'success',
            duration: UNDO_MS,
            action: undoAction(() => (done ? activitiesApi.reopen(item.id) : activitiesApi.complete(item.id))),
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

  const reassign = (target: ReassignTarget, ownerId: string) => {
    setReassigning(null)
    void change({ owner_id: ownerId }, { owner_id: target.owner_id ?? null }, tl('toast.reassigned', { title: target.title }))
  }

  const pending = busy || (toggle.isPending && toggle.variables?.id === activity.id)

  return (
    <>
      <ActivityRow
        activity={activity}
        onToggle={onToggle}
        pending={pending}
        showRecord
        actions={
          open && (
            <>
              <div className="hidden items-center gap-1 md:flex">
                <Button variant="ghost" size="sm" disabled={pending} onClick={() => postpone(1)}>
                  {tl('actions.postponeDay')}
                </Button>
                <Button variant="ghost" size="sm" disabled={pending} onClick={() => postpone(7)}>
                  {tl('actions.postponeWeek')}
                </Button>
              </div>
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="icon" disabled={pending} aria-label={tl('actions.menu', { title: activity.title })}>
                    <MoreHorizontal aria-hidden="true" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem onSelect={() => onToggle(activity, true)}>
                    <Check aria-hidden="true" />
                    {tl('actions.complete')}
                  </DropdownMenuItem>
                  <DropdownMenuItem onSelect={() => postpone(1)}>
                    <CalendarPlus aria-hidden="true" />
                    {tl('actions.postponeDay')}
                  </DropdownMenuItem>
                  <DropdownMenuItem onSelect={() => postpone(7)}>
                    <CalendarPlus aria-hidden="true" />
                    {tl('actions.postponeWeek')}
                  </DropdownMenuItem>
                  <DropdownMenuItem onSelect={() => setReassigning(activity)}>
                    <UserRoundPen aria-hidden="true" />
                    {tl('actions.reassign')}
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </>
          )
        }
      />
      <ReassignDialog item={reassigning} onOpenChange={(value) => !value && setReassigning(null)} onConfirm={reassign} />
    </>
  )
}
