import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { toast } from '@/components/ui/toast-store'
import {
  activitiesApi,
  activitiesKeys,
  useInvalidateActivities,
  type Activity,
  type InboxItem,
  type InboxQuery,
  type InboxResponse,
  type UpdateActivityRequest,
} from '@/modules/activities/api'

export interface UndoAction {
  id: number
  message: string
  run: () => Promise<unknown>
}

type InboxChange = { kind: 'remove' } | { kind: 'patch'; patch: Partial<InboxItem> }

function applyChange(data: InboxResponse | undefined, id: string, change: InboxChange): InboxResponse | undefined {
  if (!data) return data
  let removed = 0
  const groups = data.groups.map((group) => {
    if (change.kind === 'remove') {
      const items = group.items.filter((item) => item.id !== id)
      removed += group.items.length - items.length
      return { ...group, items }
    }
    return { ...group, items: group.items.map((item) => (item.id === id ? { ...item, ...change.patch } : item)) }
  })
  return { ...data, groups, total: Math.max(0, data.total - removed) }
}

let nextUndoId = 1

export function useInboxActions(query: InboxQuery) {
  const { t } = useTranslation('activities', { keyPrefix: 'inbox' })
  const queryClient = useQueryClient()
  const invalidate = useInvalidateActivities()
  const key = activitiesKeys.inbox(query)
  const [undo, setUndo] = useState<UndoAction | null>(null)
  const [pendingIds, setPendingIds] = useState<string[]>([])

  const optimistic = async (id: string, change: InboxChange) => {
    await queryClient.cancelQueries({ queryKey: key })
    const previous = queryClient.getQueryData<InboxResponse>(key)
    queryClient.setQueryData<InboxResponse | undefined>(key, (current) => applyChange(current, id, change))
    return previous
  }

  const failed = (error: unknown, previous: InboxResponse | undefined) => {
    queryClient.setQueryData(key, previous)
    const conflict = isApiError(error) && error.code === 'version_conflict'
    toast({ title: t(conflict ? 'errors.conflict' : 'errors.generic'), variant: 'destructive' })
  }

  const track = async <T,>(id: string, work: () => Promise<T>) => {
    setPendingIds((current) => [...current, id])
    try {
      return await work()
    } finally {
      setPendingIds((current) => current.filter((item) => item !== id))
      void invalidate()
    }
  }

  const offerUndo = (message: string, run: () => Promise<unknown>) => {
    const id = nextUndoId++
    setUndo({ id, message, run })
    window.setTimeout(() => setUndo((current) => (current?.id === id ? null : current)), 8000)
  }

  const completeMutation = useMutation({ mutationFn: (id: string) => activitiesApi.complete(id) })
  const updateMutation = useMutation({
    mutationFn: ({ activity, request }: { activity: Pick<Activity, 'id' | 'updated_at'>; request: UpdateActivityRequest }) =>
      activitiesApi.update(activity.id, request, activity.updated_at),
  })

  const complete = (item: InboxItem) =>
    track(item.id, async () => {
      const previous = await optimistic(item.id, { kind: 'remove' })
      try {
        await completeMutation.mutateAsync(item.id)
        offerUndo(t('toast.completed', { title: item.title }), () => activitiesApi.reopen(item.id))
      } catch (error) {
        failed(error, previous)
      }
    })

  const update = (item: InboxItem, request: UpdateActivityRequest, message: string, revert: UpdateActivityRequest) =>
    track(item.id, async () => {
      const patch: Partial<InboxItem> = {}
      if (request.due_at !== undefined) patch.due_at = request.due_at
      if (request.owner_id !== undefined) patch.owner_id = request.owner_id
      const previous = await optimistic(item.id, { kind: 'patch', patch })
      try {
        const saved = await updateMutation.mutateAsync({ activity: item, request })
        offerUndo(message, () => activitiesApi.update(saved.id, revert, saved.updated_at))
      } catch (error) {
        failed(error, previous)
      }
    })

  const runUndo = async () => {
    if (!undo) return
    const action = undo
    setUndo(null)
    try {
      await action.run()
      toast({ title: t('toast.undone'), variant: 'success' })
    } catch (error) {
      const conflict = isApiError(error) && error.code === 'version_conflict'
      toast({ title: t(conflict ? 'errors.conflict' : 'errors.generic'), variant: 'destructive' })
    } finally {
      void invalidate()
    }
  }

  return {
    complete,
    update,
    undo,
    runUndo,
    dismissUndo: () => setUndo(null),
    isPending: (id: string) => pendingIds.includes(id),
  }
}
