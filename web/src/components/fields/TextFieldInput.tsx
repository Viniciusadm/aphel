import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'
import { ariaProps, type FieldInputBaseProps } from './types'

export type TextInputKind = 'text' | 'url' | 'email'

export interface TextFieldInputProps extends FieldInputBaseProps<string> {
  kind?: TextInputKind
  maxLength?: number
  placeholder?: string
}

const inputModes: Record<TextInputKind, 'text' | 'url' | 'email'> = { text: 'text', url: 'url', email: 'email' }

export function TextFieldInput({ kind = 'text', maxLength, placeholder, ...props }: TextFieldInputProps) {
  return (
    <Input
      {...ariaProps(props)}
      type={kind === 'text' ? 'text' : kind}
      inputMode={inputModes[kind]}
      autoComplete={kind === 'email' ? 'email' : kind === 'url' ? 'url' : 'off'}
      maxLength={maxLength}
      placeholder={placeholder ?? (kind === 'url' ? 'https://' : undefined)}
      className={cn(props.className)}
      value={props.value ?? ''}
      onChange={(event) => props.onChange(event.target.value === '' ? null : event.target.value)}
    />
  )
}
