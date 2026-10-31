import type { ComponentProps } from 'react'
import { Input } from '@/components/ui/input'
import type { DecimalSeparator } from '@/lib/decimal-separator'
import { maskNumber, numberMaskAllowsDecimals, numberToCanonical, numberToDisplay, type NumberMaskKind } from '@/lib/number-mask'

export interface NumberMaskInputProps extends Omit<ComponentProps<'input'>, 'value' | 'onChange' | 'type' | 'defaultValue'> {
  kind: NumberMaskKind
  value: string
  onValueChange: (value: string) => void
  decimalSeparator?: DecimalSeparator
}

export function NumberMaskInput({
  kind,
  value,
  onValueChange,
  decimalSeparator = 'comma',
  inputMode,
  ...props
}: NumberMaskInputProps) {
  return (
    <Input
      autoComplete="off"
      {...props}
      type="text"
      inputMode={inputMode ?? (numberMaskAllowsDecimals(kind) ? 'decimal' : 'numeric')}
      value={numberToDisplay(value, decimalSeparator)}
      onChange={(event) => onValueChange(numberToCanonical(maskNumber(event.target.value, kind, decimalSeparator)))}
    />
  )
}
