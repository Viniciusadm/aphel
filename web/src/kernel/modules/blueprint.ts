import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'

export type BlueprintLabel = string | Record<string, string>

export interface BlueprintSection {
  key: string
  label: BlueprintLabel
  items: string[]
}

export function blueprintLabel(label: BlueprintLabel, language: string): string {
  if (typeof label === 'string') return label
  const primary = language.split('-')[0]
  return label[language] ?? label[primary] ?? label.en ?? Object.values(label)[0] ?? ''
}

export interface InstalledBlueprint {
  key: string
  name: string
  home: string | null
  sections: BlueprintSection[]
}

export interface BlueprintState {
  blueprint: InstalledBlueprint | null
  configuration_error: string | null
}

const BLUEPRINT_KEY = ['blueprint'] as const

export function useBlueprintState() {
  return useQuery({
    queryKey: BLUEPRINT_KEY,
    queryFn: () => api.get<BlueprintState>('/v1/blueprint'),
    staleTime: 5 * 60_000,
  })
}

export function useInstalledBlueprint() {
  const query = useBlueprintState()
  return { ...query, data: query.data?.blueprint ?? null }
}

export function useReapplyBlueprint() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: () => api.post<BlueprintState>('/v1/blueprint/configuration'),
    onSuccess: (state) => queryClient.setQueryData(BLUEPRINT_KEY, state),
  })
}
