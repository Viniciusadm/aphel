import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Briefcase, Plus } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { RecordFormDialog } from '@/features/records/form'
import { formatCurrency, useRecordFormatting } from '@/features/records/formatting'
import { dealsApi, dealsKeys } from './api'
import { DEAL_CREATE_LAYOUT, DEAL_RECORD_TYPE, dealPath } from './resource'
import type { DealStatus } from './types'

const STATUS_VARIANT: Record<DealStatus, BadgeProps['variant']> = {
  open: 'secondary',
  won: 'success',
  lost: 'destructive',
}

export interface RecordDealsTabProps {
  recordType: 'contact' | 'company'
  recordId: string
}

export function RecordDealsTab({ recordType, recordId }: RecordDealsTabProps) {
  const { t } = useTranslation('deals')
  const formatting = useRecordFormatting()
  const queryClient = useQueryClient()
  const { hasPermission } = useSession()
  const [creating, setCreating] = useState(false)
  const field = recordType === 'contact' ? 'contact_id' : 'company_id'
  const queryKey = dealsKeys.related(field, recordId)
  const deals = useQuery({ queryKey, queryFn: () => dealsApi.related(field, recordId) })
  const canCreate = hasPermission('deals.create')

  const createButton = canCreate ? (
    <Button size="sm" variant="outline" onClick={() => setCreating(true)}>
      <Plus aria-hidden="true" />
      {t('related.new')}
    </Button>
  ) : undefined

  let body
  if (deals.isPending) {
    body = (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-12 w-full" />
        <Skeleton className="h-12 w-full" />
      </div>
    )
  } else if (deals.isError) {
    body = <ErrorState title={t('related.loadError')} onRetry={() => void deals.refetch()} />
  } else if (deals.data.data.length === 0) {
    body = <EmptyState icon={Briefcase} title={t('related.empty')} action={createButton} />
  } else {
    body = (
      <div className="grid gap-3">
        <div className="flex items-center justify-between gap-2">
          <p className="text-sm text-muted-foreground">{t('related.total', { count: deals.data.meta.total })}</p>
          {createButton}
        </div>
        <ul aria-label={t('related.title')} className="grid gap-2">
          {deals.data.data.map((deal) => (
            <li key={deal.id} className="flex flex-wrap items-center justify-between gap-2 rounded-md border p-3">
              <div className="grid min-w-0 gap-0.5">
                <Link to={dealPath(deal.id)} className="truncate font-medium hover:underline">
                  {deal.title}
                </Link>
                <span className="text-xs text-muted-foreground">{deal.stage.name}</span>
              </div>
              <div className="flex items-center gap-2">
                {deal.amount != null && (
                  <span className="text-sm tabular-nums">{formatCurrency(deal.amount, formatting, deal.currency)}</span>
                )}
                <Badge variant={STATUS_VARIANT[deal.status]}>{t(`status.${deal.status}`)}</Badge>
              </div>
            </li>
          ))}
        </ul>
      </div>
    )
  }

  return (
    <>
      {body}
      {canCreate && (
        <RecordFormDialog
          recordType={DEAL_RECORD_TYPE}
          open={creating}
          onOpenChange={setCreating}
          layout={DEAL_CREATE_LAYOUT}
          defaultValues={{ [field]: recordId }}
          title={t('dialog.title')}
          description={t('dialog.description')}
          onCreated={() => {
            void queryClient.invalidateQueries({ queryKey })
            void queryClient.invalidateQueries({ queryKey: dealsKeys.boards() })
          }}
        />
      )}
    </>
  )
}
