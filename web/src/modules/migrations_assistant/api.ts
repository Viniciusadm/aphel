import { useMutation, useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type {
  MigrationFileInput,
  MigrationHistoryResult,
  MigrationImportResult,
  MigrationPreview,
  MigrationSource,
  MigrationStep,
  StepKind,
  ValueMaps,
} from './types'

const BASE_PATH = '/v1/migration-assistant'
const STORAGE_PREFIX = 'aphel.migration-assistant'

export const migrationKeys = {
  sources: ['migration-assistant', 'sources'] as const,
}

export function emptyMaps(): ValueMaps {
  return { stage_id: {}, pipeline_id: {}, owner_id: {} }
}

function formData(input: MigrationFileInput): FormData {
  const form = new FormData()
  form.append('source', input.source)
  form.append('step', input.step)
  form.append('maps', JSON.stringify(input.maps))
  form.append('file', input.file)
  return form
}

export const migrationApi = {
  sources: () => api.get<MigrationSource[]>(`${BASE_PATH}/sources`),
  preview: (input: MigrationFileInput) => api.post<MigrationPreview>(`${BASE_PATH}/preview`, formData(input)),
  createImport: (input: MigrationFileInput) =>
    api.post<MigrationImportResult>(`${BASE_PATH}/imports`, formData(input)),
  migrateHistory: (input: MigrationFileInput) =>
    api.post<MigrationHistoryResult>(`${BASE_PATH}/history`, formData(input)),
}

export function useMigrationSources() {
  return useQuery({ queryKey: migrationKeys.sources, queryFn: migrationApi.sources, staleTime: Infinity })
}

export function usePreviewMigrationFile() {
  return useMutation({ mutationFn: migrationApi.preview })
}

export function useMigrateHistory() {
  return useMutation({ mutationFn: migrationApi.migrateHistory })
}

export function useCreateMigrationImport() {
  return useMutation({ mutationFn: migrationApi.createImport })
}

function storageKey(source: string): string {
  return `${STORAGE_PREFIX}.${source}`
}

export interface StepImport {
  id: string
  rows: number | null
  sourceName: string
}

export function readStepImports(source: string): Record<string, StepImport> {
  try {
    const raw = window.localStorage.getItem(storageKey(source))
    const parsed: unknown = raw ? JSON.parse(raw) : {}
    if (typeof parsed !== 'object' || parsed === null) return {}
    return Object.fromEntries(
      Object.entries(parsed as Record<string, unknown>).map(([step, value]) => [
        step,
        typeof value === 'string'
          ? { id: value, rows: null, sourceName: '' }
          : (value as StepImport),
      ]),
    )
  } catch {
    return {}
  }
}

export function writeStepImports(source: string, imports: Record<string, StepImport>): void {
  try {
    window.localStorage.setItem(storageKey(source), JSON.stringify(imports))
  } catch {
    return
  }
}

export function stepKind(step: MigrationStep): StepKind {
  const kind = (step as MigrationStep & { kind?: string }).kind
  return kind === 'tasks' || kind === 'notes' ? kind : 'import'
}
