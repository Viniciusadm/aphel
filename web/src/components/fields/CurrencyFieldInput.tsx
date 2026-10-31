import { MoneyInput } from '@/components/ui/money-input'
import { moneyFromNumber, moneyToNumber } from '@/lib/money'
import { DEFAULT_CURRENCY_DECIMALS } from './lib/currency'
import { numberSeparators } from './lib/number'
import { ariaProps, type FieldInputBaseProps } from './types'

export interface CurrencyFieldInputProps extends FieldInputBaseProps<number> {
  locale: string
  currency: string
  decimals?: number | null
}

export function CurrencyFieldInput({ locale, decimals, ...props }: CurrencyFieldInputProps) {
  const fractionDigits = decimals ?? DEFAULT_CURRENCY_DECIMALS
  return (
    <MoneyInput
      {...ariaProps(props)}
      className={props.className}
      decimals={fractionDigits}
      separators={numberSeparators(locale)}
      allowNegative
      value={moneyFromNumber(props.value, fractionDigits)}
      onValueChange={(next) => props.onChange(moneyToNumber(next))}
    />
  )
}
