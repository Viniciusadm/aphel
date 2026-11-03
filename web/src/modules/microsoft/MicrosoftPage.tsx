import { useModuleTranslation } from '@/kernel/modules/i18n'
import { ProviderPage } from '../sso/shared/components'
import { MICROSOFT_KEY } from './api'

export function MicrosoftPage() {
  const t = useModuleTranslation(MICROSOFT_KEY)
  return (
    <ProviderPage moduleKey={MICROSOFT_KEY} title={t('page.title')} description={t('page.description')} providerName="Microsoft" />
  )
}
