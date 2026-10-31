import type { ChangeEvent, ClipboardEvent, ComponentProps, MouseEvent } from 'react'
import { useLayoutEffect, useRef } from 'react'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'
import { formatMoney, moneyFromPasted, moneyFromTyped, type MoneySeparators } from '@/lib/money'

export interface MoneyInputProps
  extends Omit<ComponentProps<'input'>, 'value' | 'onChange' | 'onPaste' | 'type' | 'ref' | 'defaultValue'> {
  value: string
  onValueChange: (value: string) => void
  separators: MoneySeparators
  decimals?: number
  allowNegative?: boolean
}

function moveCaretToEnd(input: HTMLInputElement) {
  const end = input.value.length
  input.setSelectionRange(end, end)
}

export function MoneyInput({
  value,
  onValueChange,
  separators,
  decimals = 2,
  allowNegative = false,
  className,
  onClick,
  ...props
}: MoneyInputProps) {
  const inputRef = useRef<HTMLInputElement>(null)
  const mask = { decimals, separators, allowNegative }
  const display = formatMoney(value, mask)

  useLayoutEffect(() => {
    const input = inputRef.current
    if (input && document.activeElement === input) moveCaretToEnd(input)
  }, [display])

  function handleChange(event: ChangeEvent<HTMLInputElement>) {
    const input = event.currentTarget
    const next = moneyFromTyped(input.value, display, mask)
    if (next === value) {
      requestAnimationFrame(() => moveCaretToEnd(input))
      return
    }
    onValueChange(next)
  }

  function handlePaste(event: ClipboardEvent<HTMLInputElement>) {
    const pasted = moneyFromPasted(event.clipboardData.getData('text'), mask)
    if (pasted === null) return
    event.preventDefault()
    onValueChange(pasted)
  }

  function handleClick(event: MouseEvent<HTMLInputElement>) {
    const input = event.currentTarget
    if (input.selectionStart === input.selectionEnd) moveCaretToEnd(input)
    onClick?.(event)
  }

  return (
    <Input
      autoComplete="off"
      {...props}
      ref={inputRef}
      type="text"
      inputMode={decimals === 0 && !allowNegative ? 'numeric' : 'decimal'}
      className={cn('text-right tabular-nums', className)}
      value={display}
      onChange={handleChange}
      onPaste={handlePaste}
      onClick={handleClick}
    />
  )
}
