import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'

type Schemas = components['schemas']

export type PackageTemplate = Schemas['PackageTemplateResponse']
export type PackagePlan = Schemas['PackagePlanResponse']
export type PlanItem = Schemas['PlanItemResponse']
export type PlanChange = Schemas['PlanChangeResponse']
export type PlanSummary = Schemas['PlanSummaryResponse']
export type PlanAction = Schemas['PlanActionResponse']
export type PackageIssue = Schemas['PackageIssueResponse']
export type ApplyResult = Schemas['ApplyPackageResponse']
export type PackageInstall = Schemas['ConfigPackageInstallResponse']
export type PackageInstallDetail = Schemas['ConfigPackageInstallDetailResponse']
export type PackageInstallsPage = Schemas['Page_ConfigPackageInstallResponse']
export type ConflictDecision = Schemas['ConflictDecisionBody']

export type PackageJson = Record<string, unknown>
export type PackageSource = { package: PackageJson } | { template: string }
export type Decisions = Record<string, ConflictDecision>

export interface ExportBody {
  sections: string[]
  name?: string
  description?: string
  author?: string
}

export const INSTALLS_PER_PAGE = 20
export const PLAN_OUTDATED_CODE = 'plan_outdated'

export const configPackagesKeys = {
  all: ['config-packages'] as const,
  sections: () => [...configPackagesKeys.all, 'sections'] as const,
  templates: () => [...configPackagesKeys.all, 'templates'] as const,
  installs: () => [...configPackagesKeys.all, 'installs'] as const,
  installsPage: (page: number) => [...configPackagesKeys.installs(), 'page', page] as const,
  install: (id: string) => [...configPackagesKeys.installs(), 'detail', id] as const,
}

function installPath(id: string) {
  return `/v1/config-packages/installs/${encodeURIComponent(id)}`
}

export const configPackagesApi = {
  sections: () => api.get<Schemas['ConfigSectionsResponse']>('/v1/config-packages/sections'),
  exportPackage: (body: ExportBody) => api.post<PackageJson>('/v1/config-packages/export', body),
  templates: () => api.get<PackageTemplate[]>('/v1/config-packages/templates'),
  plan: (source: PackageSource) => api.post<PackagePlan>('/v1/config-packages/plan', source),
  apply: (source: PackageSource, fingerprint: string, decisions: Decisions) =>
    api.post<ApplyResult>('/v1/config-packages/apply', { ...source, fingerprint, decisions }),
  installs: (page: number) =>
    api.get<PackageInstallsPage>('/v1/config-packages/installs', { query: { page, per_page: INSTALLS_PER_PAGE } }),
  install: (id: string) => api.get<PackageInstallDetail>(installPath(id)),
}

export function useConfigSections() {
  return useQuery({
    queryKey: configPackagesKeys.sections(),
    queryFn: async () => (await configPackagesApi.sections()).sections,
    staleTime: Infinity,
  })
}

export function usePackageTemplates() {
  return useQuery({
    queryKey: configPackagesKeys.templates(),
    queryFn: configPackagesApi.templates,
    staleTime: Infinity,
  })
}

export function usePackageInstalls(page: number) {
  return useQuery({
    queryKey: configPackagesKeys.installsPage(page),
    queryFn: () => configPackagesApi.installs(page),
    placeholderData: keepPreviousData,
  })
}

export function usePackageInstall(id: string | null) {
  return useQuery({
    queryKey: configPackagesKeys.install(id ?? ''),
    queryFn: () => configPackagesApi.install(id ?? ''),
    enabled: id !== null,
  })
}

export function useApplyPackage() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ source, fingerprint, decisions }: { source: PackageSource; fingerprint: string; decisions: Decisions }) =>
      configPackagesApi.apply(source, fingerprint, decisions),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: configPackagesKeys.installs() }),
  })
}
