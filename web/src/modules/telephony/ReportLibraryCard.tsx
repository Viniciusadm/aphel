import { useTranslation } from 'react-i18next'
import { ReportCard } from '@/components/ui/report-card'

export function CallsReportLibraryCard() {
  const { t } = useTranslation('telephony')
  return <ReportCard to="/telephony/report" title={t('reportLibrary.title')} description={t('reportLibrary.description')} />
}
