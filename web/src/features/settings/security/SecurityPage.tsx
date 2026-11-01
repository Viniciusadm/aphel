import { useTranslation } from 'react-i18next'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { ApiTokensSection } from './sessions/ApiTokensSection'
import { PasswordSection } from './sessions/PasswordSection'
import { SessionsSection } from './sessions/SessionsSection'
import { TwoFactorSection } from './two-factor/TwoFactorSection'

export function SecurityPage() {
  const { t } = useTranslation('security')
  return (
    <Page>
      <PageHeader title={t('title')} />
      <PasswordSection />
      <TwoFactorSection />
      <SessionsSection />
      <ApiTokensSection />
    </Page>
  )
}
