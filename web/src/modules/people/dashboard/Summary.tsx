import { useTranslation } from 'react-i18next'
import { SummaryMetrics } from '@/kernel/widgets/SummaryMetrics'
export function PeopleSummary({ data }: { data: Record<string, unknown> }) {
  const { t } = useTranslation('people', { keyPrefix: 'dashboard.summary' })
  return <SummaryMetrics data={data} metrics={[{ key: 'contacts_total', label: t('contactsTotal') }]} />
}
