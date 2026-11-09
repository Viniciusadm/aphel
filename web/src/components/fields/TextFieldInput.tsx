import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { MaskedInput } from '@/components/ui/masked-input'
import type { TextMaskKind } from '@/lib/text-mask'
import { cn } from '@/lib/utils'
import { ariaProps, type FieldInputBaseProps } from './types'

export type TextInputKind = 'text' | 'url' | 'email'

export interface TextFieldInputProps extends FieldInputBaseProps<string> {
  kind?: TextInputKind
  maxLength?: number
  placeholder?: string
  mask?: TextMaskKind | null
}

const inputModes: Record<TextInputKind, 'text' | 'url' | 'email'> = { text: 'text', url: 'url', email: 'email' }

export function TextFieldInput({ kind = 'text', maxLength, placeholder, mask, ...props }: TextFieldInputProps) {
  const { t } = useTranslation('records-form')
  if (mask) {
    return (
      <MaskedInput
        {...ariaProps(props)}
        mask={mask}
        maxLength={maxLength}
        placeholder={placeholder}
        className={cn(props.className)}
        value={props.value ?? ''}
        onValueChange={(next) => props.onChange(next === '' ? null : next)}
      />
    )
  }
  const defaultPlaceholder = kind === 'url' ? 'https://' : kind === 'email' ? t('input.emailPlaceholder') : t('input.textPlaceholder')
  return (
    <Input
      {...ariaProps(props)}
      type={kind === 'text' ? 'text' : kind}
      inputMode={inputModes[kind]}
      autoComplete={kind === 'email' ? 'email' : kind === 'url' ? 'url' : 'off'}
      maxLength={maxLength}
      placeholder={placeholder ?? defaultPlaceholder}
      className={cn(props.className)}
      value={props.value ?? ''}
      onChange={(event) => props.onChange(event.target.value === '' ? null : event.target.value)}
    />
  )
}
