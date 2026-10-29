import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { FailedJobsPage, JobStats, Migrations, RecentFailures, SystemOverview } from './types'

export const FAILED_JOBS_PER_PAGE = 20
const REFRESH_INTERVAL_MS = 15_000

export const systemKeys = {
  all: ['system'] as const,
  overview: () => [...systemKeys.all, 'overview'] as const,
  jobStats: () => [...systemKeys.all, 'job-stats'] as const,
  failedJobs: (kind: string, page: number) => [...systemKeys.all, 'failed-jobs', kind, page] as const,
  failures: () => [...systemKeys.all, 'failures'] as const,
  migrations: () => [...systemKeys.all, 'migrations'] as const,
}

export const systemApi = {
  overview: () => api.get<SystemOverview>('/v1/system/overview'),
  jobStats: () => api.get<JobStats>('/v1/system/jobs/stats'),
  failedJobs: (kind: string, page: number) =>
    api.get<FailedJobsPage>('/v1/system/jobs/failed', {
      query: { kind: kind || undefined, page, per_page: FAILED_JOBS_PER_PAGE },
    }),
  retryJob: (id: string) => api.post<void>(`/v1/system/jobs/${encodeURIComponent(id)}/retry`),
  discardJob: (id: string) => api.delete<void>(`/v1/system/jobs/${encodeURIComponent(id)}`),
  failures: () => api.get<RecentFailures>('/v1/system/failures'),
  migrations: () => api.get<Migrations>('/v1/system/migrations'),
}

export function useSystemOverview(enabled = true) {
  return useQuery({
    queryKey: systemKeys.overview(),
    queryFn: systemApi.overview,
    enabled,
    refetchInterval: REFRESH_INTERVAL_MS,
  })
}

export function useJobStats() {
  return useQuery({
    queryKey: systemKeys.jobStats(),
    queryFn: systemApi.jobStats,
    refetchInterval: REFRESH_INTERVAL_MS,
  })
}

export function useFailedJobs(kind: string, page: number) {
  return useQuery({
    queryKey: systemKeys.failedJobs(kind, page),
    queryFn: () => systemApi.failedJobs(kind, page),
    placeholderData: keepPreviousData,
  })
}

export function useRecentFailures() {
  return useQuery({ queryKey: systemKeys.failures(), queryFn: systemApi.failures })
}

export function useMigrations() {
  return useQuery({ queryKey: systemKeys.migrations(), queryFn: systemApi.migrations })
}

export function useJobAction(action: 'retry' | 'discard') {
  const client = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => (action === 'retry' ? systemApi.retryJob(id) : systemApi.discardJob(id)),
    onSettled: () => client.invalidateQueries({ queryKey: systemKeys.all }),
  })
}
