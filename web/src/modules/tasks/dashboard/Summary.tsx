import { useTranslation } from 'react-i18next'
import { SummaryMetrics } from '@/kernel/widgets/SummaryMetrics'
export function TasksSummary({ data }: { data: Record<string, unknown> }) {
  const { t } = useTranslation('tasks', { keyPrefix: 'dashboard.summary' })
  return <SummaryMetrics data={data} metrics={[{ key: 'pending_tasks', label: t('pendingTasks') }, { key: 'completed_activities', label: t('completedActivities') }]} />
}
