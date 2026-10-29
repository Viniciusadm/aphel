import { useModuleTranslation } from '@/kernel/modules/i18n'
import { ProviderPage } from '../oidc/shared/components'
import { GOOGLE_KEY } from './api'

export function GooglePage() {
  const t = useModuleTranslation(GOOGLE_KEY)
  return (
    <ProviderPage moduleKey={GOOGLE_KEY} title={t('page.title')} description={t('page.description')} providerName="Google" />
  )
}
