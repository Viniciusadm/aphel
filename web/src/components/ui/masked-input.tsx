import type { ComponentProps } from 'react'
import { Input } from '@/components/ui/input'
import {
  cleanTextMask,
  formatTextMask,
  textMaskInputMode,
  textMaskMaxLength,
  textMaskPlaceholder,
  type TextMaskKind,
} from '@/lib/text-mask'

export interface MaskedInputProps extends Omit<ComponentProps<'input'>, 'value' | 'onChange' | 'type' | 'defaultValue'> {
  mask: TextMaskKind
  value: string
  onValueChange: (value: string) => void
}

export function MaskedInput({ mask, value, onValueChange, inputMode, maxLength, placeholder, ...props }: MaskedInputProps) {
  return (
    <Input
      autoComplete="off"
      {...props}
      type="text"
      inputMode={inputMode ?? textMaskInputMode(mask)}
      maxLength={maxLength ?? textMaskMaxLength(mask)}
      placeholder={placeholder ?? textMaskPlaceholder(mask)}
      value={formatTextMask(cleanTextMask(value, mask), mask)}
      onChange={(event) => onValueChange(cleanTextMask(event.target.value, mask))}
    />
  )
}
