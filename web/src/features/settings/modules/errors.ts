import type { ParseKeys } from 'i18next'
import { isApiError } from '@/api'
import { notifyError } from '@/components/feedback'
import { toast } from '@/components/ui/toast-store'
import i18n from '@/i18n'

type ModulesMessageKey = ParseKeys<'modules'>

const CODE_MESSAGES: Record<string, ModulesMessageKey> = {
  module_incompatible: 'errors.module_incompatible',
  module_dependency_disabled: 'errors.module_dependency_disabled',
  module_settings_required: 'errors.module_settings_required',
  module_required_by: 'errors.module_required_by',
  module_enabled: 'errors.module_enabled',
  module_export_required: 'errors.module_export_required',
  module_confirmation_required: 'errors.module_confirmation_required',
}

export function moduleErrorMessage(error: unknown): string | null {
  if (!isApiError(error)) return null
  const key = CODE_MESSAGES[error.code]
  return key ? i18n.t(key, { ns: 'modules' }) : null
}

export function notifyModuleError(error: unknown, moduleName?: string) {
  const message = moduleErrorMessage(error)
  if (!message) {
    notifyError(error)
    return
  }
  toast({
    title: moduleName
      ? i18n.t('errors.actionFailedFor', { ns: 'modules', name: moduleName })
      : i18n.t('errors.actionFailed', { ns: 'modules' }),
    description: message,
    variant: 'destructive',
  })
}
