import { isApiError } from '@/api'
import { errorMessage } from '@/components/feedback/notify'
import { toast } from '@/components/ui/toast-store'
import i18n from '@/i18n'

const translateLoose = i18n.t.bind(i18n) as unknown as (key: string, options: object) => string

export function consoleText(key: string): string | null {
  const text = translateLoose(key, { ns: 'console', defaultValue: '' })
  return text === '' ? null : text
}

export function consoleErrorMessage(error: unknown): string {
  return (isApiError(error) ? consoleText(`errors.${error.code}`) : null) ?? errorMessage(error)
}

export function notifyConsoleError(error: unknown): void {
  toast({
    title: i18n.t('notify.errorTitle', { ns: 'records-detail' }),
    description: consoleErrorMessage(error),
    variant: 'destructive',
  })
}

export function notifyConsoleSuccess(message: string): void {
  toast({ title: message, variant: 'success' })
}
