import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'

export type Notification = components['schemas']['NotificationResponse']
export interface NotificationPage {
  data: Notification[]
  meta: { next_cursor?: string | null }
}
export type { NotificationKind } from '@/modules/notifications/ui/lib/kinds'
export type NotificationChannel = components['schemas']['NotificationChannel']
export type NotificationPreference = components['schemas']['NotificationPreferenceItem']
export type NotificationPreferences = components['schemas']['NotificationPreferencesResponse']

export const UNREAD_POLL_INTERVAL_MS = 30_000
export const NOTIFICATIONS_PAGE_SIZE = 20

export const notificationsKeys = {
  all: ['notifications'] as const,
  list: (unreadOnly: boolean, perPage: number) => [...notificationsKeys.all, 'list', { unreadOnly, perPage }] as const,
  unreadCount: () => [...notificationsKeys.all, 'unread-count'] as const,
  preferences: () => [...notificationsKeys.all, 'preferences'] as const,
}

export const notificationsApi = {
  list: (unreadOnly: boolean, perPage: number, cursor?: string | null) =>
    api.get<NotificationPage>('/v1/notifications', {
      query: { unread: unreadOnly ? true : undefined, per_page: perPage, cursor: cursor ?? undefined },
    }),
  unreadCount: () => api.get<components['schemas']['UnreadCountResponse']>('/v1/notifications/unread-count'),
  markRead: (id: string) => api.post<void>(`/v1/notifications/${encodeURIComponent(id)}/read`),
  markAllRead: () => api.post<components['schemas']['MarkAllReadResponse']>('/v1/notifications/read-all'),
  preferences: () => api.get<NotificationPreferences>('/v1/notifications/preferences'),
  updatePreferences: (preferences: NotificationPreference[]) =>
    api.put<NotificationPreferences>('/v1/notifications/preferences', { preferences }),
}

export function useUnreadNotificationCount() {
  return useQuery({
    queryKey: notificationsKeys.unreadCount(),
    queryFn: notificationsApi.unreadCount,
    refetchInterval: UNREAD_POLL_INTERVAL_MS,
    refetchIntervalInBackground: false,
    select: (response) => response.count,
  })
}

export function useNotifications(unreadOnly: boolean, perPage: number, enabled = true) {
  return useInfiniteQuery({
    queryKey: notificationsKeys.list(unreadOnly, perPage),
    queryFn: ({ pageParam }) => notificationsApi.list(unreadOnly, perPage, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (lastPage) => lastPage.meta.next_cursor ?? null,
    enabled,
  })
}

function useInvalidateNotifications() {
  const queryClient = useQueryClient()
  return () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: [...notificationsKeys.all, 'list'] }),
      queryClient.invalidateQueries({ queryKey: notificationsKeys.unreadCount() }),
    ])
}

export function useMarkNotificationRead() {
  const invalidate = useInvalidateNotifications()
  return useMutation({ mutationFn: notificationsApi.markRead, onSuccess: () => invalidate() })
}

export function useMarkAllNotificationsRead() {
  const invalidate = useInvalidateNotifications()
  return useMutation({ mutationFn: notificationsApi.markAllRead, onSuccess: () => invalidate() })
}

export function useNotificationPreferences() {
  return useQuery({ queryKey: notificationsKeys.preferences(), queryFn: notificationsApi.preferences })
}

export function useUpdateNotificationPreference() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (preference: NotificationPreference) => notificationsApi.updatePreferences([preference]),
    onMutate: async (preference) => {
      await queryClient.cancelQueries({ queryKey: notificationsKeys.preferences() })
      const previous = queryClient.getQueryData<NotificationPreferences>(notificationsKeys.preferences())
      if (previous) {
        queryClient.setQueryData<NotificationPreferences>(notificationsKeys.preferences(), {
          preferences: previous.preferences.map((item) =>
            item.kind === preference.kind && item.channel === preference.channel ? preference : item,
          ),
        })
      }
      return { previous }
    },
    onError: (_error, _preference, context) => {
      if (context?.previous) queryClient.setQueryData(notificationsKeys.preferences(), context.previous)
    },
    onSuccess: (response) => queryClient.setQueryData(notificationsKeys.preferences(), response),
  })
}
