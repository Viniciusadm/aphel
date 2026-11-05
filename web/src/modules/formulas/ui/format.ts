import { formatCurrency, formatDate, formatNumber, type RecordFormatting } from '@/sdk'
import { DEFAULT_FORMULA_CURRENCY, type FormulaResultType } from '@/sdk'
import type { FormulasT } from '@/sdk'

export interface FormulaFormatOptions {
  resultType: FormulaResultType
  decimals?: number
  currency?: string
}

function rounded(value: unknown, decimals: number | undefined): unknown {
  if (decimals === undefined || typeof value !== 'number') return value
  const factor = 10 ** decimals
  return Math.round(value * factor) / factor
}

export function formatFormulaValue(
  value: unknown,
  { resultType, decimals, currency }: FormulaFormatOptions,
  formatting: RecordFormatting,
  t: FormulasT,
): string {
  if (value === null || value === undefined || value === '') return ''
  switch (resultType) {
    case 'number':
      return formatNumber(rounded(value, decimals), formatting, decimals)
    case 'currency':
      return formatCurrency(rounded(value, decimals), formatting, currency?.trim().toUpperCase() || DEFAULT_FORMULA_CURRENCY)
    case 'date':
      return formatDate(value, formatting) || String(value)
    case 'boolean':
      return value === true ? t('preview.yes') : value === false ? t('preview.no') : String(value)
    case 'text':
      return typeof value === 'string' ? value : JSON.stringify(value)
  }
}
