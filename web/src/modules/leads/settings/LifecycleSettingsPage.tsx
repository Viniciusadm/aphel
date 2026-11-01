import { useTranslation } from 'react-i18next'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { LIFECYCLE_MANAGE_PERMISSION } from '../constants'
import { DisqualificationReasonsSection } from './DisqualificationReasonsSection'
import { TransitionRulesSection } from './TransitionRulesSection'

export function LifecycleSettingsPage() {
  const { t } = useTranslation('settings')
  return (
    <RequirePermission permission={LIFECYCLE_MANAGE_PERMISSION}>
      <Page>
        <PageHeader title={t('tabs.lifecycle')} description={t('items.lifecycle.description')} />
        <TransitionRulesSection />
        <DisqualificationReasonsSection />
      </Page>
    </RequirePermission>
  )
}
