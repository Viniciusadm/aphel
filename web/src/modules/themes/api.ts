import { useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type { ThemeList, ThemePackage, ThemeSummary, ThemeValidation } from './lib/types'

export const themesKeys = {
  all: ['themes'] as const,
  list: () => [...themesKeys.all, 'list'] as const,
  package: (key: string) => [...themesKeys.all, 'package', key] as const,
}

function themePath(key: string, suffix = '') {
  return `/v1/themes/${encodeURIComponent(key)}${suffix}`
}

export const themesApi = {
  list: () => api.get<ThemeList>('/v1/themes'),
  validate: (themePackage: ThemePackage) => api.post<ThemeValidation>('/v1/themes/validate', themePackage),
  install: (themePackage: ThemePackage) => api.post<ThemeSummary>('/v1/themes', themePackage),
  save: (key: string, themePackage: ThemePackage) => api.put<ThemeSummary>(themePath(key), themePackage),
  remove: (key: string) => api.delete<void>(themePath(key)),
  activate: (key: string | null) => api.post<ThemeList>('/v1/themes/activation', { key }),
  exportPackage: (key: string) => api.get<ThemePackage>(themePath(key, '/package')),
  downloadCatalogTheme: (id: string) =>
    api.post<ThemePackage>(`/v1/catalog/themes/${encodeURIComponent(id)}/download`, {}),
}

export function useThemes() {
  return useQuery({ queryKey: themesKeys.list(), queryFn: themesApi.list })
}
