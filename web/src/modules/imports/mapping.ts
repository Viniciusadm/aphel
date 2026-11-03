import type { DuplicateKey, ImportJob, ImportTarget } from './api'

export const LIST_PATHS: Record<string, string> = {
  'people.person': '/people',
  'organizations.organization': '/organizations',
  'deals.deal': '/deals',
  'activities.activity': '/activities',
}

type DeclaredJob = ImportJob & { duplicate_keys?: DuplicateKey[] }
type DeclaredTarget = ImportTarget & { multiple?: boolean; satisfied_by?: string | null }

export function duplicateKeysFor(job: DeclaredJob): DuplicateKey[] {
  return job.duplicate_keys?.length ? job.duplicate_keys : ['none']
}

export function acceptsMultiple(target: DeclaredTarget): boolean {
  return target.multiple === true
}

export function supportsCompanyCreation(recordType: string, targets: ImportTarget[]): boolean {
  return recordType !== 'organizations.organization' && targets.some((target) => target.key === 'company_id')
}

export function missingRequired(targets: DeclaredTarget[], mapping: (string | null)[]): ImportTarget[] {
  const mapped = new Set(mapping.filter((key): key is string => Boolean(key)))
  return targets.filter((target) => {
    if (!target.required || mapped.has(target.key)) return false
    return !(target.satisfied_by && mapped.has(target.satisfied_by))
  })
}

export function delimiterLabel(delimiter: string): string {
  return delimiter === '\\t' ? 'TAB' : delimiter
}


export interface ImportRowError {
  field?: string | null
  column?: string | null
  message: string
}

export interface ImportRowReport {
  row_index: number
  errors: ImportRowError[]
}

export function describeError(error: ImportRowError): string {
  const where = error.column ?? error.field
  return where ? `${where}: ${error.message}` : error.message
}

export function processedRows(job: ImportJob): number {
  const { imported, updated, skipped, failed } = job.totals
  return imported + updated + skipped + failed
}
