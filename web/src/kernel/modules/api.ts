import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api, type components } from '@/api'

export type ModuleInfo = components['schemas']['ModuleResponse']
export type ModuleSettingField = components['schemas']['ModuleSettingFieldResponse']
export type ModuleDataExport = components['schemas']['ModuleDataExportResponse']

export const modulesKeys = {
  all: ['modules'] as const,
  list: () => [...modulesKeys.all, 'list'] as const,
}

export const modulesApi = {
  list: () => api.get<ModuleInfo[]>('/v1/modules'),
  get: (key: string) => api.get<ModuleInfo>(`/v1/modules/${key}`),
  enable: (key: string) => api.post<ModuleInfo>(`/v1/modules/${key}/enable`),
  disable: (key: string) => api.post<ModuleInfo>(`/v1/modules/${key}/disable`),
  updateSettings: (key: string, settings: Record<string, unknown>) =>
    api.put<ModuleInfo>(`/v1/modules/${key}/settings`, { settings }),
  exportData: (key: string) => api.post<ModuleDataExport>(`/v1/modules/${key}/data-export`),
  purgeData: (key: string, confirm: string) => api.post<ModuleInfo>(`/v1/modules/${key}/data-purge`, { confirm }),
}

export function useModules() {
  return useQuery({ queryKey: modulesKeys.list(), queryFn: modulesApi.list, staleTime: 60_000 })
}

export function useInvalidateModules() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: modulesKeys.all })
}
