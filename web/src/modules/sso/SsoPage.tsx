import { useModuleTranslation } from '@/kernel/modules/i18n'
import { SSO_KEY } from './api'
import { ProviderPage } from './shared/components'

export function SsoPage() {
  const t = useModuleTranslation(SSO_KEY)
  return (
    <ProviderPage moduleKey={SSO_KEY} title={t('page.title')} description={t('page.description')} providerName="OIDC" />
  )
}
