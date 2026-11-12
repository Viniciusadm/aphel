import { useQuery, useQueryClient } from '@tanstack/react-query'
import { createContext, useContext } from 'react'
import { api, type components } from '@/api'

export type ModuleInfo = components['schemas']['ModuleResponse']
export type PublicModuleInfo = components['schemas']['PublicModuleResponse']
export type ModuleSettingField = components['schemas']['ModuleSettingFieldResponse']
export type ModuleDataExport = components['schemas']['ModuleDataExportResponse']

export const modulesKeys = {
  all: ['modules'] as const,
  list: () => [...modulesKeys.all, 'list'] as const,
  publicList: () => [...modulesKeys.all, 'public'] as const,
}

export const modulesApi = {
  list: () => api.get<ModuleInfo[]>('/v1/modules'),
  publicList: () => api.get<PublicModuleInfo[]>('/v1/public/modules'),
  get: (key: string) => api.get<ModuleInfo>(`/v1/modules/${key}`),
  enable: (key: string) => api.post<ModuleInfo>(`/v1/modules/${key}/enable`),
  disable: (key: string) => api.post<ModuleInfo>(`/v1/modules/${key}/disable`),
  updateSettings: (key: string, settings: Record<string, unknown>) =>
    api.put<ModuleInfo>(`/v1/modules/${key}/settings`, { settings }),
  exportData: (key: string) => api.post<ModuleDataExport>(`/v1/modules/${key}/data-export`),
  purgeData: (key: string, confirm: string) => api.post<ModuleInfo>(`/v1/modules/${key}/data-purge`, { confirm }),
}

export type ModulesApi = Pick<typeof modulesApi, 'enable' | 'disable' | 'updateSettings' | 'exportData' | 'purgeData'>

export const ModulesApiContext = createContext<ModulesApi>(modulesApi)

export function useModulesApi(): ModulesApi {
  return useContext(ModulesApiContext)
}

export function useModules() {
  return useQuery({ queryKey: modulesKeys.list(), queryFn: modulesApi.list, staleTime: 60_000 })
}

export function usePublicModules() {
  return useQuery({ queryKey: modulesKeys.publicList(), queryFn: modulesApi.publicList, staleTime: 60_000 })
}

export function useInvalidateModules() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: modulesKeys.all })
}
