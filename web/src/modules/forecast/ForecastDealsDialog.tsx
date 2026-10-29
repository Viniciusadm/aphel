import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Badge } from '@/components/ui/badge'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { dealPath } from '@/modules/deals/resource'
import { formatCurrency, formatDate, useRecordFormatting } from '@/features/records/formatting'
import { Pagination } from '@/features/settings/components/Pagination'
import { cn } from '@/lib/utils'
import { FORECAST_DEALS_PER_PAGE, useForecastDeals, type ForecastCategory } from './api'

export interface DrillDown {
  month: string
  label: string
  category?: ForecastCategory
}

export function ForecastDealsDialog({ drillDown, onClose }: { drillDown: DrillDown | null; onClose: () => void }) {
  const { t } = useTranslation('forecast')
  return (
    <Dialog open={drillDown !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="capitalize">
            {drillDown?.category
              ? t('deals.titleCategory', { month: drillDown.label, category: t(`categories.${drillDown.category}`) })
              : t('deals.title', { month: drillDown?.label ?? '' })}
          </DialogTitle>
          <DialogDescription>{t('deals.description')}</DialogDescription>
        </DialogHeader>
        {drillDown && <DealsList key={`${drillDown.month}-${drillDown.category ?? ''}`} drillDown={drillDown} />}
      </DialogContent>
    </Dialog>
  )
}

function DealsList({ drillDown }: { drillDown: DrillDown }) {
  const { t } = useTranslation('forecast')
  const formatting = useRecordFormatting()
  const [page, setPage] = useState(1)
  const deals = useForecastDeals({ month: drillDown.month, category: drillDown.category, page })

  if (deals.isPending) {
    return (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-14 w-full" />
      </div>
    )
  }
  if (deals.isError) return <ErrorState onRetry={() => void deals.refetch()} />
  if (deals.data.data.length === 0) return <EmptyState title={t('deals.empty')} />

  return (
    <div className="grid min-w-0 gap-3">
      <ul
        className={cn('grid max-h-[60vh] gap-2 overflow-y-auto transition-opacity', deals.isFetching && 'opacity-60')}
        aria-label={t('deals.listLabel')}
      >
        {deals.data.data.map((deal) => (
          <li key={deal.id} className="flex min-w-0 flex-col gap-1 rounded-md border p-3 sm:flex-row sm:items-center sm:justify-between">
            <div className="grid min-w-0 gap-0.5">
              <Link to={dealPath(deal.id)} className="truncate font-medium underline-offset-4 hover:underline">
                {deal.title}
              </Link>
              <p className="truncate text-xs text-muted-foreground">
                {[deal.stage_name, deal.owner?.display_name, formatDate(deal.expected_close_date, formatting)]
                  .filter(Boolean)
                  .join(' · ')}
              </p>
            </div>
            <div className="flex shrink-0 flex-wrap items-center gap-2 sm:justify-end">
              {deal.category && <Badge variant="outline">{t(`categories.${deal.category}`)}</Badge>}
              <div className="grid text-right">
                <span className="font-medium tabular-nums">
                  {deal.amount ? formatCurrency(deal.amount, formatting, deal.currency) : t('deals.noAmount')}
                </span>
                <span className="text-xs text-muted-foreground tabular-nums">
                  {t('deals.weighted', {
                    probability: deal.probability,
                    amount: formatCurrency(deal.weighted_amount ?? 0, formatting, deal.currency),
                  })}
                </span>
              </div>
            </div>
          </li>
        ))}
      </ul>
      <Pagination page={page} perPage={FORECAST_DEALS_PER_PAGE} total={deals.data.meta.total} onPageChange={setPage} />
    </div>
  )
}
