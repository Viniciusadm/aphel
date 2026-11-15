import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { CreateReminder, Reminder, RemindersPage, ReminderStatus, UpdateReminder } from './types'

export const REMINDERS_PER_PAGE = 25

export const remindersKeys = {
  all: ['reminders'] as const,
  list: (status: ReminderStatus, page: number) => [...remindersKeys.all, 'list', status, page] as const,
  record: (recordType: string, recordId: string) => [...remindersKeys.all, 'record', recordType, recordId] as const,
}

const path = (id: string) => `/v1/reminders/${encodeURIComponent(id)}`

export const remindersApi = {
  list: (query: Record<string, string | number>) => api.get<RemindersPage>('/v1/reminders', { query }),
  create: (body: CreateReminder) => api.post<Reminder>('/v1/reminders', body),
  update: (id: string, body: UpdateReminder) => api.patch<Reminder>(path(id), body),
  remove: (id: string) => api.delete<void>(path(id)),
}

export function useReminders(status: ReminderStatus, page: number) {
  return useQuery({
    queryKey: remindersKeys.list(status, page),
    queryFn: () => remindersApi.list({ status, page, per_page: REMINDERS_PER_PAGE }),
    placeholderData: keepPreviousData,
  })
}

export function useRecordReminders(recordType: string, recordId: string) {
  return useQuery({
    queryKey: remindersKeys.record(recordType, recordId),
    queryFn: () => remindersApi.list({ status: 'pending', record_type: recordType, record_id: recordId, per_page: 50 }),
  })
}

function useInvalidate() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: remindersKeys.all })
}

export function useCreateReminder() {
  const invalidate = useInvalidate()
  return useMutation({ mutationFn: remindersApi.create, onSuccess: invalidate })
}

export function useUpdateReminder() {
  const invalidate = useInvalidate()
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: UpdateReminder }) => remindersApi.update(id, body),
    onSuccess: invalidate,
  })
}

export function useDeleteReminder() {
  const invalidate = useInvalidate()
  return useMutation({ mutationFn: remindersApi.remove, onSuccess: invalidate })
}
