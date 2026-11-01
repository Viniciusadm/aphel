import { useTranslation } from 'react-i18next'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { ReportPage } from './ReportPage'

export function CallsReportPage() {
  const { t } = useTranslation('calls')
  return <Page><PageHeader title={t('page.report')} description={t('page.description')} /><ReportPage /></Page>
}
