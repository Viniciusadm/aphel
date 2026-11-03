import { useTranslation } from 'react-i18next'
import { SummaryMetrics } from '@/kernel/widgets/SummaryMetrics'
export function DealsSummary({ data }: { data: Record<string, unknown> }) {
  const { t } = useTranslation('deals', { keyPrefix: 'dashboard.summary' })
  return <SummaryMetrics data={data} metrics={[{ key: 'open_deals', label: t('openDeals') }, { key: 'won_deals', label: t('wonDeals') }, { key: 'lost_deals', label: t('lostDeals') }]} />
}
