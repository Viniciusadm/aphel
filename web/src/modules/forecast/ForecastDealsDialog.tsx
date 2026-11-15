import { PaginationSummary } from '@/components/ui/pagination-summary'
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
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { formatCurrency, formatDate, recordDetailHref, useRecordFormatting } from '@/sdk'
import { Pagination } from '@/components/ui/pagination'
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
      <DialogContent size="2xl">
        <DialogHeader>
          <DialogTitle>
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
      <PaginationSummary page={page} perPage={FORECAST_DEALS_PER_PAGE} total={deals.data.meta.total} />
      <div className={cn('max-h-[60vh] overflow-y-auto rounded-lg transition-opacity', deals.isFetching && 'opacity-60')}>
        <Table>
          <caption className="sr-only">{t('deals.listLabel')}</caption>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">{t('deals.columns.deal')}</TableHead>
              <TableHead scope="col" className="hidden sm:table-cell">{t('deals.columns.stage')}</TableHead>
              <TableHead scope="col" className="hidden sm:table-cell">{t('deals.columns.closeDate')}</TableHead>
              <TableHead scope="col" className="text-right">{t('deals.columns.amount')}</TableHead>
              <TableHead scope="col" className="hidden text-right sm:table-cell">{t('deals.columns.weighted')}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {deals.data.data.map((deal) => (
              <TableRow key={deal.id}>
                <TableCell className="max-w-40 sm:max-w-56">
                  <Link to={recordDetailHref('deals.deal', deal.id) ?? '#'} className="block truncate font-medium underline-offset-4 hover:underline">
                    {deal.title}
                  </Link>
                  {deal.owner?.display_name && <span className="block truncate text-xs text-muted-foreground">{deal.owner.display_name}</span>}
                  {deal.category && !drillDown.category && <Badge variant="outline" className="mt-1">{t(`categories.${deal.category}`)}</Badge>}
                </TableCell>
                <TableCell className="hidden sm:table-cell">{deal.stage_name}</TableCell>
                <TableCell className="hidden tabular-nums sm:table-cell">{formatDate(deal.expected_close_date, formatting)}</TableCell>
                <TableCell className="text-right font-medium tabular-nums">
                  {deal.amount ? formatCurrency(deal.amount, formatting, deal.currency) : t('deals.noAmount')}
                </TableCell>
                <TableCell className="hidden text-right tabular-nums sm:table-cell">
                  {formatCurrency(deal.weighted_amount ?? 0, formatting, deal.currency)}
                  <span className="block text-xs text-muted-foreground">{deal.probability}%</span>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
      <Pagination page={page} perPage={FORECAST_DEALS_PER_PAGE} total={deals.data.meta.total} onPageChange={setPage} />
    </div>
  )
}
