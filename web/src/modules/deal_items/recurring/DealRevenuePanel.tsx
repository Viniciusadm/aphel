import { ArrowRight, Repeat } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useRecord } from '@/features/records/detail'
import { formatCurrency, formatDate, useRecordFormatting } from '@/features/records/formatting'
import { DEAL_RECORD_TYPE, dealPath } from '@/modules/deals/resource'
import { useLinkedDeal, useRenewalOf, type LinkedDeal } from './api'
import type { DealRevenueFields } from './types'

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex min-w-0 flex-col gap-1 rounded-lg border bg-background p-3">
      <dt className="truncate text-xs font-medium text-muted-foreground">{label}</dt>
      <dd className="truncate text-lg font-semibold tabular-nums">{value}</dd>
    </div>
  )
}

function DealLink({ label, deal }: { label: string; deal: LinkedDeal }) {
  return (
    <p className="flex min-w-0 flex-wrap items-center gap-2 text-sm">
      <span className="text-muted-foreground">{label}</span>
      <Link
        className="inline-flex min-w-0 items-center gap-1 font-medium text-primary hover:underline"
        to={dealPath(deal.id)}
      >
        <span className="truncate">{deal.title}</span>
        <ArrowRight className="size-3.5 shrink-0" aria-hidden="true" />
      </Link>
    </p>
  )
}

export function DealRevenuePanel({ dealId }: { dealId: string }) {
  const { t } = useTranslation('deal_items')
  const formatting = useRecordFormatting()
  const record = useRecord(DEAL_RECORD_TYPE, dealId)
  const deal = (record.data ?? {}) as DealRevenueFields
  const renewal = useRenewalOf(dealId)
  const original = useLinkedDeal(deal.renewal_of_deal_id)

  if (record.isPending) return <Skeleton className="h-32 w-full" />
  if (record.isError) return <ErrorState onRetry={() => void record.refetch()} />

  const money = (value: string | null | undefined) =>
    value ? formatCurrency(value, formatting, deal.currency) : t('panel.empty')
  const day = (value: string | null | undefined) => (value ? formatDate(value, formatting) : t('panel.empty'))
  const hasRevenue = Boolean(deal.mrr || deal.tcv || deal.contract_start || deal.contract_end)
  const links = (
    <>
      {original.data && <DealLink label={t('panel.renewalOf')} deal={original.data} />}
      {renewal.data && <DealLink label={t('panel.renewedBy')} deal={renewal.data} />}
    </>
  )

  if (!hasRevenue) {
    return (
      <div className="grid gap-3">
        <EmptyState icon={Repeat} title={t('panel.emptyTitle')} description={t('panel.emptyDescription')} />
        {links}
      </div>
    )
  }

  return (
    <div className="grid gap-4">
      <dl className="grid grid-cols-1 gap-3 sm:grid-cols-3">
        <Metric label={t('panel.mrr')} value={money(deal.mrr)} />
        <Metric label={t('panel.arr')} value={money(deal.arr)} />
        <Metric label={t('panel.tcv')} value={money(deal.tcv)} />
      </dl>
      <dl className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <Metric label={t('panel.contractStart')} value={day(deal.contract_start)} />
        <Metric label={t('panel.contractEnd')} value={day(deal.contract_end)} />
      </dl>
      <p className="text-xs text-muted-foreground">{t('panel.hint')}</p>
      {links}
    </div>
  )
}
