import { isApiError } from '@/api'
import { type FieldViolation, fieldViolations } from '@/sdk'
import type { AutomationDefinition } from '../types'

export const INVALID_AUTOMATION = 'invalid_automation'
export const VERSION_CONFLICT = 'version_conflict'

export interface ValidationIssue {
  path: string
  message: string
}

export interface ValidationReport {
  general: ValidationIssue[]
  trigger: ValidationIssue[]
  details: ValidationIssue[]
  steps: Record<string, ValidationIssue[]>
}

export const EMPTY_REPORT: ValidationReport = { general: [], trigger: [], details: [], steps: {} }

const STEP_PATH = /^definition\.steps\.(\d+)(?:\.(.*))?$/

export function isValidationError(error: unknown): boolean {
  return isApiError(error) && (error.code === INVALID_AUTOMATION || (error.status === 422 && fieldViolations(error.details).length > 0))
}

export function buildReport(violations: FieldViolation[], definition: AutomationDefinition): ValidationReport {
  const report: ValidationReport = { general: [], trigger: [], details: [], steps: {} }
  for (const violation of violations) {
    const match = STEP_PATH.exec(violation.field)
    if (match) {
      const stepId = definition.steps[Number(match[1])]?.id
      if (stepId) {
        const list = report.steps[stepId] ?? []
        list.push({ path: match[2] ?? '', message: violation.message })
        report.steps[stepId] = list
        continue
      }
    }
    if (violation.field === 'trigger' || violation.field.startsWith('trigger.')) {
      report.trigger.push({ path: violation.field.replace(/^trigger\.?/, ''), message: violation.message })
    } else if (violation.field === 'name' || violation.field === 'description' || violation.field === 'key') {
      report.details.push({ path: violation.field, message: violation.message })
    } else {
      report.general.push({ path: violation.field, message: violation.message })
    }
  }
  return report
}

export function reportFromError(error: unknown, definition: AutomationDefinition): ValidationReport | null {
  if (!isApiError(error)) return null
  const violations = fieldViolations(error.details)
  if (violations.length === 0) return null
  return buildReport(violations, definition)
}

export function issueCount(report: ValidationReport): number {
  return (
    report.general.length +
    report.trigger.length +
    report.details.length +
    Object.values(report.steps).reduce((total, list) => total + list.length, 0)
  )
}

export function issuesUnder(issues: ValidationIssue[] | undefined, prefix: string): ValidationIssue[] {
  if (!issues) return []
  return issues
    .filter((issue) => issue.path === prefix || issue.path.startsWith(`${prefix}.`))
    .map((issue) => ({ ...issue, path: issue.path.slice(prefix.length).replace(/^\./, '') }))
}

export function issuesOutside(issues: ValidationIssue[] | undefined, prefixes: string[]): ValidationIssue[] {
  if (!issues) return []
  return issues.filter(
    (issue) => !prefixes.some((prefix) => issue.path === prefix || issue.path.startsWith(`${prefix}.`)),
  )
}
