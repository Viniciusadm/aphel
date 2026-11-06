import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { NavigationPreferences, PersonalPreferences, PreferencesResponse, UpdatePersonalPreferences } from './types'

export const EMPTY_NAVIGATION: NavigationPreferences = { order: [], hidden: [], home: null }

const PREFERENCES_QUERY_KEY = ['preferences'] as const

export const personalPreferencesApi = {
  get: () => api.get<PreferencesResponse>('/v1/preferences'),
  update: (request: UpdatePersonalPreferences) => api.patch<PreferencesResponse>('/v1/preferences', request),
}

export function normalizePreferences(preferences: PreferencesResponse | undefined): PersonalPreferences {
  return {
    keyboard_shortcuts: preferences?.keyboard_shortcuts ?? true,
    locale: preferences?.locale ?? null,
    navigation: {
      order: preferences?.navigation?.order ?? [],
      hidden: preferences?.navigation?.hidden ?? [],
      home: preferences?.navigation?.home ?? null,
    },
  }
}

export function usePersonalPreferences() {
  return useQuery({
    queryKey: PREFERENCES_QUERY_KEY,
    queryFn: personalPreferencesApi.get,
    staleTime: 5 * 60_000,
    select: (data) => normalizePreferences(data),
  })
}

export function useNavigationPreferences(): NavigationPreferences {
  return usePersonalPreferences().data?.navigation ?? EMPTY_NAVIGATION
}

export function useUpdatePersonalPreferences() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: personalPreferencesApi.update,
    onSuccess: (preferences) => queryClient.setQueryData(PREFERENCES_QUERY_KEY, preferences),
  })
}
