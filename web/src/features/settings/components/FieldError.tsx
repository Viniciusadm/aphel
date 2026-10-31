import { useTranslation } from 'react-i18next'
import { asMessageKey } from '../lib/errors'
import { Alert } from '@/components/ui/alert'

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
    <Alert className="text-destructive">
      {t(key)}
    </Alert>
  )
}
