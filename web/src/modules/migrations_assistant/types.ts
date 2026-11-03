import type { components } from '@/api'

type Schemas = components['schemas']

export type StepKey = 'organizations' | 'people' | 'deals' | 'tasks' | 'notes'
export type StepKind = 'import' | 'tasks' | 'notes'
export type DateOrder = 'day_first' | 'month_first'
export type DecimalSeparator = 'comma' | 'dot'

export type MigrationColumn = Schemas['MigrationColumnResponse']
export type MigrationStep = Schemas['MigrationStepResponse']
export type MigrationSource = Schemas['MigrationSourceResponse']
export type RecognizedColumn = Schemas['RecognizedColumnResponse']
export type MigrationPreview = Schemas['MigrationPreviewResponse']
export type MigrationImportResult = Schemas['MigrationImportResponse']

export interface ValueMaps {
  stage_id: Record<string, string>
  pipeline_id: Record<string, string>
  owner_id: Record<string, string>
}

export type MapTarget = keyof ValueMaps

export interface MigrationFileInput {
  source: string
  step: string
  file: File
  maps: ValueMaps
}

export interface MigrationHistoryReport {
  total: number
  created: number
  skipped: number
  failed: number
  errors: { row: number; message: string }[]
}

export interface MigrationHistoryResult {
  report: MigrationHistoryReport
  preview: MigrationPreview
}
