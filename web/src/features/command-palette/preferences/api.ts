import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'

export type UserPreferences = components['schemas']['PreferencesResponse']
export type UpdateUserPreferences = components['schemas']['UpdatePreferencesRequest']

export const DEFAULT_PREFERENCES: UserPreferences = {
  ui_plugin: null,
  ui_plugin_default: null,
  effective_ui_plugin: null,
  keyboard_shortcuts: true,
  locale: null,
  navigation: { order: [], hidden: [], home: null },
}

export const preferencesKeys = {
  all: ['preferences'] as const,
}

export const preferencesApi = {
  get: () => api.get<UserPreferences>('/v1/preferences'),
  update: (request: UpdateUserPreferences) => api.patch<UserPreferences>('/v1/preferences', request),
}

export function usePreferences() {
  return useQuery({ queryKey: preferencesKeys.all, queryFn: preferencesApi.get, staleTime: 5 * 60_000 })
}

export function useUpdatePreferences() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: preferencesApi.update,
    onSuccess: (preferences) => queryClient.setQueryData(preferencesKeys.all, preferences),
  })
}
