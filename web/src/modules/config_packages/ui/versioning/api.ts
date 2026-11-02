import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import { configPackagesKeys, type PackageJson } from '@/modules/config_packages/ui/api'
import type { InstalledVersion, Resolutions, VersionApplyResult, VersionComparison } from '@/modules/config_packages/ui/versioning/types'

export const versioningKeys = {
  all: ['config-package-versions'] as const,
  list: () => [...versioningKeys.all, 'list'] as const,
}

export const versioningApi = {
  list: () => api.get<InstalledVersion[]>('/v1/config-package-versions'),
  compare: (pkg: PackageJson) => api.post<VersionComparison>('/v1/config-package-versions/compare', { package: pkg }),
  apply: (pkg: PackageJson, fingerprint: string, resolutions: Resolutions) =>
    api.post<VersionApplyResult>('/v1/config-package-versions/apply', { package: pkg, fingerprint, resolutions }),
}

export function useInstalledVersions() {
  return useQuery({ queryKey: versioningKeys.list(), queryFn: versioningApi.list })
}

export function useInvalidateVersions() {
  const queryClient = useQueryClient()
  return () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: versioningKeys.all }),
      queryClient.invalidateQueries({ queryKey: configPackagesKeys.installs() }),
    ])
}

export interface InstalledPackage {
  latest: InstalledVersion
  history: InstalledVersion[]
}

export function groupByPackage(versions: InstalledVersion[]): InstalledPackage[] {
  const groups = new Map<string, InstalledVersion[]>()
  for (const version of versions) {
    groups.set(version.package_id, [...(groups.get(version.package_id) ?? []), version])
  }
  return [...groups.values()].map(([latest, ...history]) => ({ latest, history }))
}
