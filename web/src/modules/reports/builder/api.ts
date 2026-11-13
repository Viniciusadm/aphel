import { useTranslation } from 'react-i18next'
import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { API_BASE_URL, ApiError, CSRF_HEADER, CSRF_HEADER_VALUE, NETWORK_ERROR_CODE, UNKNOWN_ERROR_CODE, api, parseErrorEnvelope } from '@/api'
import type { components } from '@/modules/reports/api-types'
import { type AggregateResult } from '@/sdk'
type Schemas = components['schemas']

export type ReportSettings = Record<string, unknown>
export type Report = Omit<Schemas['ReportResponse'], 'settings'> & { settings: ReportSettings }
export type ReportVisualization = Schemas['ReportVisualization']
export type ReportVisibility = Schemas['ViewVisibility']
export type CreateReportRequest = Omit<Schemas['CreateReportRequest'], 'settings'> & { settings?: ReportSettings }
export type UpdateReportRequest = Omit<Schemas['UpdateReportRequest'], 'settings'> & { settings?: ReportSettings }
export type RunReportRequest = Schemas['RunReportRequest']
export type ReportSchedule = Schemas['ReportScheduleResponse']
export type ReportScheduleFrequency = Schemas['ReportScheduleFrequency']
export type CreateScheduleRequest = Schemas['CreateScheduleRequest']
export type UpdateScheduleRequest = Schemas['UpdateScheduleRequest']
export type ReportDelivery = Schemas['ReportDeliveryResponse']
export type PeriodPreset = Schemas['DashboardPeriodPreset']
export type ExportFormat = 'csv' | 'xlsx'

interface Page<T> {
  data: T[]
  meta: Schemas['PageMeta']
}

const PER_PAGE = 100
const MAX_PAGES = 20

export const reportKeys = {
  all: ['reports', 'saved'] as const,
  list: () => [...reportKeys.all, 'list'] as const,
  detail: (id: string) => [...reportKeys.all, 'detail', id] as const,
  run: (id: string, request: RunReportRequest) => [...reportKeys.all, 'run', id, request] as const,
  schedules: (id: string) => [...reportKeys.all, 'schedules', id] as const,
  deliveries: (id: string, scheduleId: string) => [...reportKeys.all, 'deliveries', id, scheduleId] as const,
}

function reportPath(id: string, suffix = '') {
  return `/v1/reports/${encodeURIComponent(id)}${suffix}`
}

function versionHeader(version?: string | null) {
  return version ? { 'If-Match': version } : undefined
}

async function listAll<T>(path: string): Promise<T[]> {
  const items: T[] = []
  for (let page = 1; page <= MAX_PAGES; page += 1) {
    const result = await api.get<Page<T>>(path, { query: { page, per_page: PER_PAGE } })
    items.push(...result.data)
    if (page * PER_PAGE >= result.meta.total) break
  }
  return items
}

export const reportsApi = {
  list: () => listAll<Report>('/v1/reports'),
  get: (id: string) => api.get<Report>(reportPath(id)),
  create: (request: CreateReportRequest) => api.post<Report>('/v1/reports', request),
  update: (id: string, request: UpdateReportRequest, version?: string | null) =>
    api.patch<Report>(reportPath(id), request, { headers: versionHeader(version) }),
  remove: (id: string) => api.delete<void>(reportPath(id)),
  run: (id: string, request: RunReportRequest) => api.post<AggregateResult>(reportPath(id, '/run'), request),
  schedules: (id: string) => listAll<ReportSchedule>(reportPath(id, '/schedules')),
  createSchedule: (id: string, request: CreateScheduleRequest) =>
    api.post<ReportSchedule>(reportPath(id, '/schedules'), request),
  updateSchedule: (id: string, scheduleId: string, request: UpdateScheduleRequest, version?: string | null) =>
    api.patch<ReportSchedule>(reportPath(id, `/schedules/${encodeURIComponent(scheduleId)}`), request, {
      headers: versionHeader(version),
    }),
  removeSchedule: (id: string, scheduleId: string) =>
    api.delete<void>(reportPath(id, `/schedules/${encodeURIComponent(scheduleId)}`)),
  deliveries: (id: string, scheduleId: string) =>
    api.get<Page<ReportDelivery>>(reportPath(id, `/schedules/${encodeURIComponent(scheduleId)}/deliveries`), {
      query: { page: 1, per_page: 20 },
    }),
}

function fileNameFrom(disposition: string | null, fallback: string): string {
  if (!disposition) return fallback
  const encoded = /filename\*=UTF-8''([^;]+)/i.exec(disposition)
  if (encoded) {
    try {
      return decodeURIComponent(encoded[1])
    } catch {
      return fallback
    }
  }
  const plain = /filename="?([^";]+)"?/i.exec(disposition)
  return plain ? plain[1] : fallback
}

export async function downloadReport(id: string, format: ExportFormat, request: RunReportRequest): Promise<void> {
  const params = new URLSearchParams({ format })
  if (request.preset) params.set('preset', request.preset)
  if (request.from) params.set('from', request.from)
  if (request.to) params.set('to', request.to)
  if (request.compare_to_previous !== undefined && request.compare_to_previous !== null) {
    params.set('compare_to_previous', String(request.compare_to_previous))
  }
  const headers = new Headers({ [CSRF_HEADER]: CSRF_HEADER_VALUE })
  let response: Response
  try {
    response = await fetch(`${API_BASE_URL}${reportPath(id, '/export')}?${params.toString()}`, {
      headers,
      credentials: 'include',
    })
  } catch (cause) {
    throw new ApiError(0, { code: NETWORK_ERROR_CODE, message: cause instanceof Error ? cause.message : String(cause) })
  }
  if (!response.ok) {
    const text = await response.text()
    let payload: unknown = text
    try {
      payload = JSON.parse(text)
    } catch {
      payload = text
    }
    throw new ApiError(
      response.status,
      parseErrorEnvelope(payload) ?? { code: UNKNOWN_ERROR_CODE, message: response.statusText },
    )
  }
  const blob = await response.blob()
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = fileNameFrom(response.headers.get('Content-Disposition'), `relatorio.${format}`)
  document.body.appendChild(link)
  link.click()
  link.remove()
  URL.revokeObjectURL(url)
}

export function useReports() {
  return useQuery({ queryKey: reportKeys.list(), queryFn: reportsApi.list })
}

export function useReport(id: string | null) {
  return useQuery({
    queryKey: reportKeys.detail(id ?? ''),
    queryFn: () => reportsApi.get(id as string),
    enabled: Boolean(id),
  })
}

export function useReportRun(id: string | null, request: RunReportRequest, enabled = true) {
  return useQuery({
    queryKey: reportKeys.run(id ?? '', request),
    queryFn: () => reportsApi.run(id as string, request),
    enabled: Boolean(id) && enabled,
    placeholderData: keepPreviousData,
    retry: false,
  })
}

export function useReportMutations() {
  const queryClient = useQueryClient()
  const store = (report: Report) => {
    queryClient.setQueryData(reportKeys.detail(report.id), report)
    return queryClient.invalidateQueries({ queryKey: reportKeys.list() })
  }
  const create = useMutation({ mutationFn: reportsApi.create, onSuccess: store })
  const update = useMutation({
    mutationFn: ({ id, request, version }: { id: string; request: UpdateReportRequest; version?: string | null }) =>
      reportsApi.update(id, request, version),
    onSuccess: store,
  })
  const remove = useMutation({
    mutationFn: reportsApi.remove,
    onSuccess: (_, id) => {
      queryClient.removeQueries({ queryKey: reportKeys.detail(id) })
      return queryClient.invalidateQueries({ queryKey: reportKeys.list() })
    },
  })
  return { create, update, remove }
}

export function useReportSchedules(id: string) {
  return useQuery({ queryKey: reportKeys.schedules(id), queryFn: () => reportsApi.schedules(id) })
}

export function useScheduleDeliveries(id: string, scheduleId: string | null) {
  return useQuery({
    queryKey: reportKeys.deliveries(id, scheduleId ?? ''),
    queryFn: () => reportsApi.deliveries(id, scheduleId as string),
    enabled: Boolean(scheduleId),
  })
}

export function useScheduleMutations(id: string) {
  const queryClient = useQueryClient()
  const invalidate = () => queryClient.invalidateQueries({ queryKey: reportKeys.schedules(id) })
  const create = useMutation({
    mutationFn: (request: CreateScheduleRequest) => reportsApi.createSchedule(id, request),
    onSuccess: invalidate,
  })
  const update = useMutation({
    mutationFn: ({ schedule, request }: { schedule: ReportSchedule; request: UpdateScheduleRequest }) =>
      reportsApi.updateSchedule(id, schedule.id, request, schedule.updated_at),
    onSuccess: invalidate,
  })
  const remove = useMutation({
    mutationFn: (scheduleId: string) => reportsApi.removeSchedule(id, scheduleId),
    onSuccess: invalidate,
  })
  return { create, update, remove }
}

export interface ReportEntity {
  recordType: string
  label: string
}

export function useReportEntities() {
  const { i18n } = useTranslation()
  return useQuery({
    queryKey: ['reports', 'entities', i18n.language],
    queryFn: async (): Promise<ReportEntity[]> => {
      const entities = await api.get<Schemas['ReportEntityResponse'][]>('/v1/reports/entities')
      return entities
        .map((entity) => ({ recordType: entity.record_type,
          label: entity.labels[i18n.language] ?? entity.labels[i18n.language.split('-')[0]] ?? entity.labels.en ?? Object.values(entity.labels)[0] ?? entity.record_type }))
        .sort((left, right) => left.label.localeCompare(right.label, i18n.language))
    },
  })
}
