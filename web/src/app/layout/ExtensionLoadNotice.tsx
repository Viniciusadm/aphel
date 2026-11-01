import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback'
import { failedWebPlugins } from '@/kernel/modules/plugins'

let announced = false

export function ExtensionLoadNotice() {
  const { t } = useTranslation('layout')
  useEffect(() => {
    const failed = failedWebPlugins()
    if (announced || failed.length === 0) return
    announced = true
    notifyError(t('extensionsFailed', { names: failed.join(', ') }), { title: t('extensionsFailedTitle'), duration: 15000 })
  }, [t])
  return null
}
