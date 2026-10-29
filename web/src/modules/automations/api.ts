import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { Page } from '@/features/records/types'
import type {
  Automation,
  AutomationCatalog,
  AutomationListFilters,
  AutomationPayload,
  AutomationRun,
  AutomationRunDetails,
  AutomationVersion,
  CreateAutomationPayload,
  DryRunRequest,
  DryRunResult,
  RunListFilters,
} from './types'

export const AUTOMATIONS_PATH = '/automations'
export const AUTOMATIONS_PER_PAGE = 25
export const RUNS_PER_PAGE = 20

const BASE = '/v1/automations'

export function automationPath(id: string) {
  return `${AUTOMATIONS_PATH}/${encodeURIComponent(id)}`
}

function resourcePath(id: string, suffix = '') {
  return `${BASE}/${encodeURIComponent(id)}${suffix}`
}

function ifMatch(version?: string | null) {
  return version ? { 'If-Match': version } : undefined
}

export const automationsKeys = {
  all: ['automations'] as const,
  catalog: () => [...automationsKeys.all, 'catalog'] as const,
  lists: () => [...automationsKeys.all, 'list'] as const,
  list: (filters: AutomationListFilters) => [...automationsKeys.lists(), filters] as const,
  detail: (id: string) => [...automationsKeys.all, 'detail', id] as const,
  versions: (id: string) => [...automationsKeys.all, 'versions', id] as const,
  runs: (id: string) => [...automationsKeys.all, 'runs', id] as const,
  runList: (id: string, filters: RunListFilters) => [...automationsKeys.runs(id), 'list', filters] as const,
  run: (id: string, runId: string) => [...automationsKeys.runs(id), 'detail', runId] as const,
}

export const automationsApi = {
  catalog: () => api.get<AutomationCatalog>(`${BASE}/catalog`),
  list: (filters: AutomationListFilters) =>
    api.get<Page<Automation>>(BASE, {
      query: {
        status: filters.status,
        record_type: filters.record_type,
        search: filters.search || undefined,
        page: filters.page,
        per_page: AUTOMATIONS_PER_PAGE,
      },
    }),
  get: (id: string) => api.get<Automation>(resourcePath(id)),
  create: (body: CreateAutomationPayload) => api.post<Automation>(BASE, body),
  update: (id: string, body: AutomationPayload, version?: string | null) =>
    api.patch<Automation>(resourcePath(id), body, { headers: ifMatch(version) }),
  remove: (id: string) => api.delete<void>(resourcePath(id)),
  publish: (id: string, version?: string | null) =>
    api.post<Automation>(resourcePath(id, '/publish'), undefined, { headers: ifMatch(version) }),
  pause: (id: string) => api.post<Automation>(resourcePath(id, '/pause')),
  activate: (id: string) => api.post<Automation>(resourcePath(id, '/activate')),
  dryRun: (id: string, body: DryRunRequest) => api.post<DryRunResult>(resourcePath(id, '/dry-run'), body),
  run: (id: string, recordId: string) => api.post<AutomationRun>(resourcePath(id, '/run'), { record_id: recordId }),
  versions: (id: string) => api.get<AutomationVersion[]>(resourcePath(id, '/versions')),
  runs: (id: string, filters: RunListFilters) =>
    api.get<Page<AutomationRun>>(resourcePath(id, '/runs'), {
      query: {
        status: filters.status,
        record_id: filters.record_id,
        page: filters.page,
        per_page: RUNS_PER_PAGE,
      },
    }),
  runDetails: (id: string, runId: string) =>
    api.get<AutomationRunDetails>(resourcePath(id, `/runs/${encodeURIComponent(runId)}`)),
}

export function useAutomationCatalog() {
  return useQuery({
    queryKey: automationsKeys.catalog(),
    queryFn: automationsApi.catalog,
    staleTime: 5 * 60_000,
  })
}

export function useAutomations(filters: AutomationListFilters) {
  return useQuery({
    queryKey: automationsKeys.list(filters),
    queryFn: () => automationsApi.list(filters),
    placeholderData: keepPreviousData,
  })
}

export function useAutomation(id: string) {
  return useQuery({ queryKey: automationsKeys.detail(id), queryFn: () => automationsApi.get(id) })
}

export function useAutomationVersions(id: string) {
  return useQuery({ queryKey: automationsKeys.versions(id), queryFn: () => automationsApi.versions(id) })
}

export function useAutomationRuns(id: string, filters: RunListFilters) {
  return useQuery({
    queryKey: automationsKeys.runList(id, filters),
    queryFn: () => automationsApi.runs(id, filters),
    placeholderData: keepPreviousData,
  })
}

export function useAutomationRun(id: string, runId: string | null) {
  return useQuery({
    queryKey: automationsKeys.run(id, runId ?? ''),
    queryFn: () => automationsApi.runDetails(id, runId!),
    enabled: Boolean(runId),
  })
}

export function useAutomationCache() {
  const queryClient = useQueryClient()
  return {
    store: (automation: Automation) => {
      queryClient.setQueryData(automationsKeys.detail(automation.id), automation)
      return queryClient.invalidateQueries({ queryKey: automationsKeys.lists() })
    },
    invalidateAll: () => queryClient.invalidateQueries({ queryKey: automationsKeys.all }),
    invalidateRuns: (id: string) => queryClient.invalidateQueries({ queryKey: automationsKeys.runs(id) }),
    invalidateVersions: (id: string) => queryClient.invalidateQueries({ queryKey: automationsKeys.versions(id) }),
  }
}
