import { FilePlus2, FileText, ListPlus } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { formatCurrency, formatDate, useRecordFormatting } from '@/sdk'
import { quotePath, useDealQuotes, useQuoteMutations } from './api'
import { notifyQuoteError } from './errors'
import { QuoteStatusBadge } from './QuoteStatusBadge'

export const QUOTES_TAB_KEY = 'quotes'

export interface DealQuotesTabProps {
  dealId: string
}

export function DealQuotesTab({ dealId }: DealQuotesTabProps) {
  const { t } = useTranslation('quotes')
  const formatting = useRecordFormatting()
  const navigate = useNavigate()
  const { hasPermission } = useSession()
  const query = useDealQuotes(dealId)
  const mutations = useQuoteMutations()
  const canCreate = hasPermission('quotes.create')

  const create = async (fromItems: boolean) => {
    try {
      const quote = await mutations.create.mutateAsync({ deal_id: dealId, from_line_items: fromItems })
      notifySuccess(t('deal.created'))
      void navigate(quotePath(quote.id))
    } catch (error) {
      notifyQuoteError(error)
    }
  }

  const actions = canCreate && (
    <div className="flex flex-wrap gap-2">
      <Button size="sm" onClick={() => void create(true)} disabled={mutations.create.isPending}>
        <ListPlus aria-hidden="true" />
        {t('deal.fromItems')}
      </Button>
      <Button size="sm" variant="outline" onClick={() => void create(false)} disabled={mutations.create.isPending}>
        <FilePlus2 aria-hidden="true" />
        {t('deal.blank')}
      </Button>
    </div>
  )

  if (query.isPending) {
    return (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-14 w-full" />
      </div>
    )
  }
  if (query.isError) {
    return <ErrorState title={t('deal.loadError')} onRetry={() => void query.refetch()} />
  }
  const quotes = query.data
  return (
    <div className="grid gap-4">
      {actions}
      {quotes.length === 0 ? (
        <EmptyState icon={FileText} title={t('deal.empty')} />
      ) : (
        <ul className="grid gap-2">
          {quotes.map((quote) => (
            <li key={quote.id}>
              <Link
                to={quotePath(quote.id)}
                className="flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-card p-3 hover:bg-muted/50"
              >
                <div className="grid gap-0.5">
                  <span className="text-xs text-muted-foreground">
                    {t('numberVersion', { number: quote.number, version: quote.version })}
                  </span>
                  <span className="font-medium">{quote.title}</span>
                </div>
                <div className="flex items-center gap-3">
                  {quote.valid_until && (
                    <span className="text-xs text-muted-foreground">{formatDate(quote.valid_until, formatting)}</span>
                  )}
                  <span className="font-medium tabular-nums">{formatCurrency(quote.total, formatting, quote.currency)}</span>
                  <QuoteStatusBadge status={quote.status} />
                </div>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
