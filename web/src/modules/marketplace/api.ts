import { useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type {Resolutions, VersionApplyResult, VersionComparison} from '@/sdk'
import type { Catalog } from './types'

export const catalogKeys = {
  all: ['catalog'] as const,
  index: () => [...catalogKeys.all, 'index'] as const,
}

function templatePath(id: string, action: 'compare' | 'apply') {
  return `/v1/catalog/templates/${encodeURIComponent(id)}/${action}`
}

export const catalogApi = {
  get: () => api.get<Catalog>('/v1/catalog'),
  compare: (id: string) => api.post<VersionComparison>(templatePath(id, 'compare'), {}),
  apply: (id: string, fingerprint: string, resolutions: Resolutions) =>
    api.post<VersionApplyResult>(templatePath(id, 'apply'), { fingerprint, resolutions }),
}

export function useCatalog() {
  return useQuery({ queryKey: catalogKeys.index(), queryFn: catalogApi.get, retry: false, staleTime: 60_000 })
}
