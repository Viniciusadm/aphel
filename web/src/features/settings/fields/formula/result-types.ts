import type { components } from '@/api'

export type FormulaResultType = components['schemas']['FormulaResultType']
export type FormulaValueKind = components['schemas']['FormulaValueKind']

export const FORMULA_RESULT_TYPES = ['number', 'currency', 'date', 'text', 'boolean'] as const satisfies readonly FormulaResultType[]

export const DEFAULT_FORMULA_RESULT_TYPE: FormulaResultType = 'number'
export const DEFAULT_FORMULA_CURRENCY = 'BRL'
export const FORMULA_EXPRESSION_MAX_LENGTH = 2000
export const FORMULA_PREVIEW_DEBOUNCE_MS = 400

export function isFormulaResultType(value: unknown): value is FormulaResultType {
  return typeof value === 'string' && (FORMULA_RESULT_TYPES as readonly string[]).includes(value)
}

export function formulaUsesDecimals(resultType: FormulaResultType): boolean {
  return resultType === 'number' || resultType === 'currency'
}

export function formulaUsesCurrency(resultType: FormulaResultType): boolean {
  return resultType === 'currency'
}

export function parseDecimals(value: string): number | undefined {
  const trimmed = value.trim()
  if (!/^\d+$/.test(trimmed)) return undefined
  return Number(trimmed)
}
