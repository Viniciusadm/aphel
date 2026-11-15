import { ArrowRight, Repeat } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useIsModuleActive } from '@/kernel/modules/registry'
import { StatTile } from '@/kernel/widgets/shared'
import { formatCurrency, formatDate, recordDetailHref, useRecord, useRecordFormatting } from '@/sdk'
import { useLinkedDeal, useRenewalOf, type LinkedDeal } from './api'
import type { DealRevenueFields } from './types'

const DEAL_RECORD_TYPE = 'deals.deal'

function DealLink({ label, deal }: { label: string; deal: LinkedDeal }) {
  return (
    <div className="grid min-w-0 gap-0.5">
      <dt className="text-xs text-muted-foreground">{label}</dt>
      <dd className="min-w-0">
        <Link
          className="inline-flex min-w-0 items-center gap-1 font-medium text-primary hover:underline"
          to={recordDetailHref('deals.deal', deal.id) ?? '#'}
        >
          <span className="truncate">{deal.title}</span>
          <ArrowRight className="size-3.5 shrink-0" aria-hidden="true" />
        </Link>
      </dd>
    </div>
  )
}

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid min-w-0 gap-0.5">
      <dt className="text-xs text-muted-foreground">{label}</dt>
      <dd className="font-medium tabular-nums">{value}</dd>
    </div>
  )
}

export function DealRevenuePanel({ dealId }: { dealId: string }) {
  const { t } = useTranslation('deals_revenue')
  const formatting = useRecordFormatting()
  const record = useRecord(DEAL_RECORD_TYPE, dealId)
  const deal = (record.data ?? {}) as DealRevenueFields
  const renewalsActive = useIsModuleActive('renewals')
  const renewal = useRenewalOf(renewalsActive ? dealId : '')
  const original = useLinkedDeal(deal.renewal_of_deal_id)

  if (record.isPending)
    return (
      <div className="grid gap-4" aria-busy="true">
        <Skeleton className="h-36 w-full" />
        <Skeleton className="h-28 w-full" />
      </div>
    )
  if (record.isError) return <ErrorState onRetry={() => void record.refetch()} />

  const money = (value: string | null | undefined) =>
    value ? formatCurrency(value, formatting, deal.currency) : t('panel.empty')
  const day = (value: string | null | undefined) => (value ? formatDate(value, formatting) : t('panel.empty'))
  const hasRevenue = Boolean(deal.mrr || deal.tcv || deal.contract_start || deal.contract_end)
  const hasLinks = Boolean(original.data || renewal.data)
  const links = hasLinks && (
    <Card>
      <CardHeader>
        <CardTitle>{t('panel.renewalTitle')}</CardTitle>
        <CardDescription>{t('panel.renewalDescription')}</CardDescription>
      </CardHeader>
      <CardContent>
        <dl className="grid gap-4 sm:grid-cols-2">
          {original.data && <DealLink label={t('panel.renewalOf')} deal={original.data} />}
          {renewal.data && <DealLink label={t('panel.renewedBy')} deal={renewal.data} />}
        </dl>
      </CardContent>
    </Card>
  )

  if (!hasRevenue) {
    return (
      <div className="grid gap-4">
        <EmptyState icon={Repeat} title={t('panel.emptyTitle')} description={t('panel.emptyDescription')} />
        {links}
      </div>
    )
  }

  return (
    <div className="grid gap-4">
      <Card>
        <CardHeader>
          <CardTitle>{t('panel.recurringTitle')}</CardTitle>
          <CardDescription>{t('panel.recurringDescription')}</CardDescription>
        </CardHeader>
        <CardContent>
          <dl className="grid grid-cols-1 gap-3 sm:grid-cols-3">
            <StatTile label={t('panel.mrr')} value={money(deal.mrr)} />
            <StatTile label={t('panel.arr')} value={money(deal.arr)} />
            <StatTile label={t('panel.tcv')} value={money(deal.tcv)} />
          </dl>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>{t('panel.contractTitle')}</CardTitle>
          <CardDescription>{t('panel.contractDescription')}</CardDescription>
        </CardHeader>
        <CardContent>
          <dl className="grid gap-4 sm:grid-cols-2">
            <Fact label={t('panel.contractStart')} value={day(deal.contract_start)} />
            <Fact label={t('panel.contractEnd')} value={day(deal.contract_end)} />
          </dl>
        </CardContent>
      </Card>
      {links}
    </div>
  )
}
