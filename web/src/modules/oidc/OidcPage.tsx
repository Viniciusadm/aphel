import { useModuleTranslation } from '@/kernel/modules/i18n'
import { OIDC_KEY } from './api'
import { ProviderPage } from './shared/components'

export function OidcPage() {
  const t = useModuleTranslation(OIDC_KEY)
  return (
    <ProviderPage moduleKey={OIDC_KEY} title={t('page.title')} description={t('page.description')} providerName="OIDC" />
  )
}
