import { useTranslation } from 'react-i18next'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { OutcomesPage } from './OutcomesPage'

export function CallOutcomesSettingsPage() {
  const { t } = useTranslation('calls')
  return <RequirePermission permission="settings.manage"><Page><PageHeader title={t('page.outcomes')} /><OutcomesPage /></Page></RequirePermission>
}
