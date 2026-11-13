import { useTranslation } from 'react-i18next'
import { ReportCard } from '@/components/ui/report-card'

export function DealReportLibraryCards() {
  const { t } = useTranslation('deals')
  return (['conversion', 'performance'] as const).map((key) => (
    <ReportCard key={key} to={`/deals/reports/${key}`} title={t(`reportLibrary.${key}.title`)} description={t(`reportLibrary.${key}.description`)} />
  ))
}
