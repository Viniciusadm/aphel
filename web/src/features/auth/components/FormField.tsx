import type { ComponentProps } from 'react'
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { PasswordInput } from '@/components/ui/password-input'
import { asAuthMessageKey } from '../lib/messages'
import { Field } from '@/components/ui/field'

interface FormFieldProps extends ComponentProps<'input'> {
  id: string
  label: string
  error?: string
  hint?: string
}

export function FormField({ id, label, error, hint, type, ...inputProps }: FormFieldProps) {
  const { t } = useTranslation('auth')
  const errorKey = asAuthMessageKey(error)
  return (
    <Field id={id} label={label} error={errorKey ? t(errorKey) : undefined} description={!errorKey ? hint : undefined}>
      {(control) =>
        type === 'password' ? <PasswordInput {...inputProps} {...control} /> : <Input {...inputProps} type={type} {...control} />
      }
    </Field>
  )
}
