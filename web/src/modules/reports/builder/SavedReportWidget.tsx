import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import type { AggregateResult } from '@/features/records/aggregates'
import type { Report } from './api'
import { ReportResultView } from './ReportResultView'

export interface SavedReportWidgetData {
  report_id: string | null
  use_dashboard_period: boolean
  period: { preset: string; from: string; to: string }
  status: 'ok' | 'no_report' | 'unavailable'
  report: Report | null
  result: AggregateResult | null
}

export function SavedReportWidget({ data }: { data: SavedReportWidgetData }) {
  const { t } = useTranslation('reports', { keyPrefix: 'builder' })

  if (!data.report_id || data.status === 'no_report') {
    return (
      <p className="text-sm text-muted-foreground">
        {t('widget.noReport')}{' '}
        <Link to="/reports/saved" className="text-primary underline-offset-4 hover:underline">
          {t('widget.openReports')}
        </Link>
      </p>
    )
  }
  if (data.status === 'unavailable') return <p className="text-sm text-muted-foreground">{t('widget.unavailable')}</p>
  if (!data.report || !data.result) return <p className="text-sm text-muted-foreground">{t('widget.error')}</p>
  return (
    <div className="grid gap-2">
      <Link to={`/reports/saved/${data.report.id}`} className="text-sm font-medium hover:underline">
        {data.report.name}
      </Link>
      <ReportResultView
        result={data.result}
        visualization={data.report.visualization}
        chartMetric={typeof data.report.settings.chart_metric === 'string' ? data.report.settings.chart_metric : null}
        compact
      />
    </div>
  )
}
