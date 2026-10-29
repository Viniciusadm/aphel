import { CircleAlert } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { asAuthMessageKey } from '../lib/messages'

export function FormAlert({ message, values }: { message?: string; values?: Record<string, unknown> }) {
  const { t } = useTranslation('auth')
  const key = asAuthMessageKey(message)
  if (!key) return null
  return (
    <div
      role="alert"
      className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive"
    >
      <CircleAlert className="mt-0.5 size-4 shrink-0" aria-hidden />
      <span>{values ? t(key, values) : t(key)}</span>
    </div>
  )
}
