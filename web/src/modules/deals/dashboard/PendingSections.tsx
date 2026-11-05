import { Hourglass } from 'lucide-react'
import { Link } from 'react-router'
import { useTranslation } from 'react-i18next'
import { BucketSection } from '@/kernel/widgets/BucketSection'
import { formatCurrency, formatDateTime, useRecordFormatting } from '@/sdk'
import type { StaleLineItem } from './types'

export interface DealPendingData { stale_deals: { count: number; stale_days: number; items: StaleLineItem[] } | null }
export function DealPendingSections({ data }: { data: DealPendingData }) {
  const { t } = useTranslation('deals', { keyPrefix: 'dashboard' })
  const formatting = useRecordFormatting()
  const bucket = data.stale_deals
  if (!bucket) return null
  return <BucketSection id="pending-stale" icon={Hourglass} title={t('widgets.pending_center.staleDeals', { count: bucket.stale_days })} bucket={bucket} emptyLabel={t('widgets.pending_center.noItems')} moreLabel={(count) => t('widgets.pending_center.more', { count })} renderItem={(item) => <div className="grid min-w-0 gap-0.5">
    <Link to={`/deals/${encodeURIComponent(item.id)}`} className="truncate font-medium hover:underline">{item.title || item.id}</Link>
    <p className="truncate text-xs text-muted-foreground">{item.stage_name}{item.amount != null && ` · ${formatCurrency(item.amount, formatting, item.currency)}`} · {t('widgets.pending_center.lastTouched', { date: formatDateTime(item.last_touched_at, formatting) })}</p>
  </div>} />
}
