import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { formatNumberDraft, parseLocaleNumber } from './lib/number'
import { ariaProps, type FieldInputBaseProps } from './types'

export interface NumberFieldInputProps extends FieldInputBaseProps<number | string> {
  locale: string
  decimals?: number | null
}

export function NumberFieldInput({ locale, decimals, ...props }: NumberFieldInputProps) {
  const { t } = useTranslation('records-form')
  const { value, onChange } = props
  const [draft, setDraft] = useState(() => formatNumberDraft(value, locale))
  const [lastValue, setLastValue] = useState(value)

  if (!Object.is(value, lastValue)) {
    setLastValue(value)
    const parsed = parseLocaleNumber(draft, locale)
    const current = parsed === undefined ? draft : parsed
    if (!Object.is(current, value)) setDraft(formatNumberDraft(value, locale))
  }

  return (
    <Input
      {...ariaProps(props)}
      type="text"
      inputMode={decimals === 0 ? 'numeric' : 'decimal'}
      autoComplete="off"
      placeholder={t('input.numberPlaceholder')}
      className={props.className}
      value={draft}
      onChange={(event) => {
        const text = event.target.value
        setDraft(text)
        const parsed = parseLocaleNumber(text, locale)
        onChange(parsed === undefined ? text : parsed)
      }}
      onBlur={() => {
        const parsed = parseLocaleNumber(draft, locale)
        if (parsed !== undefined) setDraft(formatNumberDraft(parsed, locale))
        props.onBlur?.()
      }}
    />
  )
}
