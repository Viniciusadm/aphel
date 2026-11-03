import { Delta, StatTile } from './shared'
import { formatNumber, useRecordFormatting } from '@/features/records/formatting'

export function SummaryMetrics({ data, metrics }: { data: Record<string, unknown>; metrics: { key: string; label: string }[] }) {
  const formatting = useRecordFormatting()
  const count = (value: number) => formatNumber(value, formatting)
  const previous = data.previous as Record<string, number | null> | null | undefined
  return <dl className="grid grid-cols-2 gap-3">{metrics.map(({ key, label }) => {
    const value = data[key]
    if (typeof value !== 'number' || !Number.isFinite(value)) return null
    const prior = previous?.[key]
    return <StatTile key={key} label={label} value={count(value)} delta={typeof prior === 'number' ? <Delta current={value} previous={prior} format={count} /> : undefined} />
  })}</dl>
}
