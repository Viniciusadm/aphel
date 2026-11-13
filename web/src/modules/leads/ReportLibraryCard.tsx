import { useTranslation } from 'react-i18next'
import { ReportCard } from '@/components/ui/report-card'
import { LEADS_REPORT_PATH } from './constants'

export function LeadsReportLibraryCard() {
  const { t } = useTranslation('leads')
  return <ReportCard to={LEADS_REPORT_PATH} title={t('reportLibrary.title')} description={t('reportLibrary.description')} />
}
