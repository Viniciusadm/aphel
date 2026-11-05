import { useTranslation } from 'react-i18next'
import { formatCurrency, formatNumber, useRecordFormatting } from '@/sdk'
import type { SalesIndicatorsData } from './types'
import { Delta, StatTile } from '@/kernel/widgets/shared'

function amountOf(value: string | undefined): number | null {
  if (value === undefined) return null
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}

export function SalesIndicatorsWidget({ data }: { data: SalesIndicatorsData }) {
  const { t } = useTranslation('deals', { keyPrefix: 'dashboard' })
  const formatting = useRecordFormatting()
  const money = (value: string | number) => formatCurrency(value, formatting, data.currency)
  const count = (value: number) => formatNumber(value, formatting)
  const percent = (value: number) =>
    new Intl.NumberFormat(formatting.locale, { style: 'percent', maximumFractionDigits: 1 }).format(value)
  const rate = data.conversion_rate === null ? t('states.noData') : percent(data.conversion_rate)
  const previous = data.previous
  return (
    <dl className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
      <StatTile label={t('widgets.sales_indicators.openCount')} value={count(data.open_count)} />
      <StatTile label={t('widgets.sales_indicators.pipelineAmount')} value={data.pipeline_amount !== undefined ? money(data.pipeline_amount) : t('states.noData')} />
      <StatTile
        label={t('widgets.sales_indicators.forecastAmount')}
        value={data.forecast_amount !== undefined ? money(data.forecast_amount) : t('states.noData')}
        hint={t('widgets.sales_indicators.forecastHint')}
      />
      <StatTile
        label={t('widgets.sales_indicators.won')}
        value={count(data.won_count)}
        hint={data.won_amount !== undefined ? t('widgets.sales_indicators.wonAmount', { amount: money(data.won_amount) }) : undefined}
        delta={
          previous && (
            <Delta
              current={amountOf(data.won_amount) ?? data.won_count}
              previous={amountOf(previous.won_amount) ?? previous.won_count}
              format={previous.won_amount !== undefined ? money : count}
            />
          )
        }
      />
      <StatTile
        label={t('widgets.sales_indicators.lost')}
        value={count(data.lost_count)}
        delta={previous && <Delta current={data.lost_count} previous={previous.lost_count} format={count} inverse />}
      />
      <StatTile
        label={t('widgets.sales_indicators.conversionRate')}
        value={rate}
        delta={
          previous && <Delta current={data.conversion_rate} previous={previous.conversion_rate} format={percent} points />
        }
      />
    </dl>
  )
}
