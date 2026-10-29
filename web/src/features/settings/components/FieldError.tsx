import { useTranslation } from 'react-i18next'
import { asMessageKey } from '../lib/errors'

interface FieldErrorProps {
  id: string
  message?: string
}

export function FieldError({ id, message }: FieldErrorProps) {
  const { t } = useTranslation('settings')
  const key = asMessageKey(message)
  if (!key) return null
  return (
    <p id={id} className="text-sm text-destructive">
      {t(key)}
    </p>
  )
}

interface FormAlertProps {
  message?: string
}

export function FormAlert({ message }: FormAlertProps) {
  const { t } = useTranslation('settings')
  const key = asMessageKey(message)
  if (!key) return null
  return (
    <p role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
      {t(key)}
    </p>
  )
}
