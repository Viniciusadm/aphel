import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { isCustomRecordType } from '@/features/custom-records/resources'
import { api } from '@/api'
import type { components } from '@/api/schema'

type Schemas = components['schemas']

export type ImportJob = Schemas['ImportJobResponse']
export type ImportTarget = Schemas['ImportTargetResponse']
export type ImportOptions = Schemas['ImportOptionsDto']
export type DuplicateKey = Schemas['DuplicateKeyDto']
export type DuplicateStrategy = Schemas['DuplicateStrategyDto']
export type ImportValidation = Schemas['ImportValidationResponse']
export type ImportRow = Schemas['ImportRowResponse']
export type ImportRowsPage = Schemas['Page_ImportRowResponse']
export type ImportJobsPage = Schemas['Page_ImportJobResponse']

export const IMPORTABLE_RECORD_TYPES = ['contact', 'company', 'deal', 'product'] as const
export const POLL_INTERVAL_MS = 2000
export const ERROR_ROWS_PAGE_SIZE = 50
export const HISTORY_PAGE_SIZE = 25

export type ImportableRecordType = (typeof IMPORTABLE_RECORD_TYPES)[number]

const PERMISSION_PREFIX: Record<ImportableRecordType, string> = {
  contact: 'contacts',
  company: 'companies',
  deal: 'deals',
  product: 'products',
}

export function isImportable(recordType: string): recordType is ImportableRecordType {
  return (IMPORTABLE_RECORD_TYPES as readonly string[]).includes(recordType)
}

export function supportsImport(recordType: string): boolean {
  return isImportable(recordType) || isCustomRecordType(recordType)
}

export function importPermission(recordType: string): string | null {
  if (isCustomRecordType(recordType)) return `${recordType}.create`
  return isImportable(recordType) ? `${PERMISSION_PREFIX[recordType]}.create` : null
}

export function isRunning(status: string | undefined): boolean {
  return status === 'queued' || status === 'running'
}

export function isFinished(status: string | undefined): boolean {
  return status === 'completed' || status === 'failed'
}

export const importsKeys = {
  all: ['imports'] as const,
  detail: (id: string) => [...importsKeys.all, 'detail', id] as const,
  history: (page: number) => [...importsKeys.all, 'history', page] as const,
  rows: (id: string, status: string, page: number) => [...importsKeys.all, 'rows', id, status, page] as const,
}

function importPath(id: string, suffix = '') {
  return `/v1/imports/${encodeURIComponent(id)}${suffix}`
}

export interface UploadImportInput {
  recordType: string
  file: File
  retryOf?: string
}

export const importsApi = {
  upload: ({ recordType, file, retryOf }: UploadImportInput) => {
    const form = new FormData()
    form.append('record_type', recordType)
    if (retryOf) form.append('retry_of', retryOf)
    form.append('file', file, file.name)
    return api.post<ImportJob>('/v1/imports', form)
  },
  get: (id: string) => api.get<ImportJob>(importPath(id)),
  list: (page: number) =>
    api.get<ImportJobsPage>('/v1/imports', { query: { page, per_page: HISTORY_PAGE_SIZE } }),
  configure: (id: string, mapping: (string | null)[], options: ImportOptions) =>
    api.put<ImportJob>(importPath(id, '/mapping'), { mapping, options }),
  validate: (id: string) => api.post<ImportValidation>(importPath(id, '/validate')),
  start: (id: string) => api.post<ImportJob>(importPath(id, '/start')),
  retryFailed: (id: string) => api.post<ImportJob>(importPath(id, '/retry-failed')),
  rows: (id: string, status: string, page: number) =>
    api.get<ImportRowsPage>(importPath(id, '/rows'), {
      query: { status, page, per_page: ERROR_ROWS_PAGE_SIZE },
    }),
}

export function useImportJob(id: string | undefined) {
  return useQuery({
    queryKey: importsKeys.detail(id ?? ''),
    queryFn: () => importsApi.get(id ?? ''),
    enabled: Boolean(id),
    refetchInterval: (query) => (isRunning(query.state.data?.status) ? POLL_INTERVAL_MS : false),
  })
}

export function useImportHistory(page: number) {
  return useQuery({
    queryKey: importsKeys.history(page),
    queryFn: () => importsApi.list(page),
    placeholderData: keepPreviousData,
    refetchInterval: (query) =>
      query.state.data?.data.some((job) => isRunning(job.status)) ? POLL_INTERVAL_MS : false,
  })
}

export function useImportRows(id: string, status: string, page: number, enabled: boolean) {
  return useQuery({
    queryKey: importsKeys.rows(id, status, page),
    queryFn: () => importsApi.rows(id, status, page),
    enabled,
  })
}

function useStoreJob() {
  const queryClient = useQueryClient()
  return (job: ImportJob) => {
    queryClient.setQueryData(importsKeys.detail(job.id), job)
    void queryClient.invalidateQueries({ queryKey: [...importsKeys.all, 'rows', job.id] })
  }
}

export function useUploadImport() {
  const store = useStoreJob()
  return useMutation({ mutationFn: importsApi.upload, onSuccess: store })
}

export function useConfigureImport(id: string) {
  const store = useStoreJob()
  return useMutation({
    mutationFn: ({ mapping, options }: { mapping: (string | null)[]; options: ImportOptions }) =>
      importsApi.configure(id, mapping, options),
    onSuccess: store,
  })
}

export function useValidateImport(id: string) {
  return useMutation({ mutationFn: () => importsApi.validate(id) })
}

export function useStartImport(id: string) {
  const store = useStoreJob()
  return useMutation({ mutationFn: () => importsApi.start(id), onSuccess: store })
}

export function useRetryFailed(id: string) {
  const store = useStoreJob()
  return useMutation({ mutationFn: () => importsApi.retryFailed(id), onSuccess: store })
}
