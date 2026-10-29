import { useState } from 'react'
import { Input } from '@/components/ui/input'
import { DEFAULT_CURRENCY_DECIMALS, MAX_CURRENCY_DIGITS, formatCurrencyInput } from './lib/currency'
import { formatNumberDraft, parseLocaleNumber, toNumber } from './lib/number'
import { ariaProps, type FieldInputBaseProps } from './types'

export interface CurrencyFieldInputProps extends FieldInputBaseProps<number> {
  locale: string
  currency: string
  decimals?: number | null
}

const ALLOWED_CHARACTERS = /[^\d.,\s  -]/g

function parseCurrencyDraft(text: string, locale: string, decimals: number): number | null | undefined {
  const cleaned = text.replace(ALLOWED_CHARACTERS, '')
  const parsed = parseLocaleNumber(cleaned.trim() === '-' ? '' : cleaned, locale)
  if (parsed === null || parsed === undefined) return parsed
  const rounded = Number(parsed.toFixed(decimals))
  if (rounded !== parsed) return undefined
  if (Math.trunc(Math.abs(rounded)).toString().length + decimals > MAX_CURRENCY_DIGITS) return undefined
  return rounded
}

export function CurrencyFieldInput({ locale, currency, decimals, ...props }: CurrencyFieldInputProps) {
  const fractionDigits = decimals ?? DEFAULT_CURRENCY_DECIMALS
  const [draft, setDraft] = useState<string | null>(null)
  const formatted = formatCurrencyInput(props.value, { locale, currency, decimals: fractionDigits })
  const current = toNumber(props.value)
  const draftMatches = draft !== null && parseCurrencyDraft(draft, locale, fractionDigits) === current
  const display = draft === null ? formatted : draftMatches ? draft : formatNumberDraft(props.value, locale)

  return (
    <Input
      {...ariaProps(props)}
      type="text"
      inputMode={fractionDigits === 0 ? 'numeric' : 'decimal'}
      autoComplete="off"
      className={props.className}
      value={display}
      onFocus={() => setDraft(formatNumberDraft(props.value, locale))}
      onChange={(event) => {
        const text = event.target.value.replace(ALLOWED_CHARACTERS, '')
        const parsed = parseCurrencyDraft(text, locale, fractionDigits)
        if (parsed === undefined) return
        setDraft(text)
        if (parsed !== current) props.onChange(parsed)
      }}
      onBlur={() => {
        setDraft(null)
        props.onBlur?.()
      }}
    />
  )
}
