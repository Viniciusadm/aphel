import type { ParseKeys, TFunction } from 'i18next'
import { isApiError } from '@/api'
import { settingsFieldViolations as fieldViolations } from '@/sdk'
import type { FormulaError } from '@/modules/formulas/ui/api'

export type FormulasT = TFunction<'formulas'>
type FormulasKey = ParseKeys<'formulas'>

const ERROR_CODES = new Set([
  'unexpected_character',
  'unterminated_string',
  'invalid_number',
  'unexpected_token',
  'unexpected_end',
  'expression_too_long',
  'expression_too_complex',
  'empty_expression',
  'unknown_field',
  'unknown_relation',
  'unknown_function',
  'unsupported_field',
  'wrong_arguments',
  'type_mismatch',
  'related_outside_aggregate',
  'incompatible_result',
  'formula_cycle',
  'field_not_accessible',
])

const SNIPPET_CODES = new Set([
  'unexpected_character',
  'invalid_number',
  'unexpected_token',
  'unknown_field',
  'unknown_relation',
  'unknown_function',
  'unsupported_field',
  'related_outside_aggregate',
  'field_not_accessible',
])
const SNIPPET_MAX_LENGTH = 40

function backendDetail(message: string): string {
  return message.replace(/^at position \d+:\s*/i, '').trim()
}

function snippetOf(error: FormulaError, expression: string): string {
  const snippet = expression.slice(error.start, Math.max(error.end, error.start)).trim()
  return snippet.length > SNIPPET_MAX_LENGTH ? `${snippet.slice(0, SNIPPET_MAX_LENGTH)}…` : snippet
}

export function formulaErrorMessage(error: FormulaError, t: FormulasT, expression: string): string {
  if (!ERROR_CODES.has(error.code)) return backendDetail(error.message) || error.message
  const translated = t(`errors.${error.code}` as FormulasKey)
  if (error.code === 'formula_cycle') {
    const detail = backendDetail(error.message)
    return detail ? `${translated} (${detail})` : translated
  }
  const snippet = SNIPPET_CODES.has(error.code) ? snippetOf(error, expression) : ''
  return snippet ? `${translated} (${snippet})` : translated
}

const EXPRESSION_PATHS = new Set(['config.expression', 'config.result_type'])

export function formulaSaveMessages(error: unknown): string[] {
  if (!isApiError(error)) return []
  return fieldViolations(error.details)
    .filter((violation) => EXPRESSION_PATHS.has(violation.field))
    .map((violation) => violation.message)
}

export function functionDescription(name: string, t: FormulasT, exists: (key: string) => boolean): string | undefined {
  const key = `functions.${name}`
  return exists(`formulas:${key}`) ? t(key as FormulasKey) : undefined
}
