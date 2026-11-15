import { NativeSelect } from '@/components/ui/native-select'
import { FileText } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Button } from '@/components/ui/button'
import { Pagination } from '@/components/ui/pagination'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { formatCurrency, formatDate, formatDateTime, useRecordFormatting } from '@/sdk'
import { QUOTES_PER_PAGE, quotePath, useQuotes } from './api'
import { QuoteStatusBadge } from './QuoteStatusBadge'
import type { QuoteStatus } from './types'
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Page } from '@/components/ui/page'
import { SearchInput } from '@/components/ui/search-input'

const ALL = '__all__'
const STATUSES: QuoteStatus[] = ['draft', 'pending_approval', 'approved', 'sent', 'accepted', 'rejected', 'expired']

export function QuotesPage() {
  const { t } = useTranslation('quotes')
  const formatting = useRecordFormatting()
  const [page, setPage] = useState(1)
  const [search, setSearch] = useState('')
  const [status, setStatus] = useState('')
  const query = useQuotes(page, search.trim(), status)
  const total = query.data?.meta.total ?? 0

  return (
    <Page>
      <PageHeader title={t('title')} description={t('description')} />
      <div className="flex flex-wrap gap-2">
        <SearchInput wrapperClassName="w-full max-w-xs"
            
            placeholder={t('list.search')}
            aria-label={t('list.search')}
            value={search}
            onValueChange={(value) => {
              setSearch(value)
              setPage(1)
            }}
          />
        <NativeSelect value={status || ALL}
          onValueChange={(value) => {
            setStatus(value === ALL ? '' : value)
            setPage(1)
          }} className="w-48" aria-label={t('list.columns.status')}>
            <option value={ALL}>{t('list.allStatuses')}</option>
            {STATUSES.map((value) => (
              <option key={value} value={value}>
                {t(`status.${value}`)}
              </option>
            ))}
          </NativeSelect>
      </div>
      {query.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </div>
      ) : query.isError ? (
        <ErrorState title={t('list.loadError')} onRetry={() => void query.refetch()} />
      ) : query.data.data.length === 0 ? (
        <EmptyState
          icon={FileText}
          title={search.trim() || status ? t('list.empty') : t('list.emptyNone')}
          description={search.trim() || status ? undefined : t('list.emptyDescription')}
          action={
            search.trim() || status ? undefined : (
              <Button asChild size="sm">
                <Link to="/deals">{t('list.goToDeals')}</Link>
              </Button>
            )
          }
        />
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t('list.columns.number')}</TableHead>
              <TableHead>{t('list.columns.title')}</TableHead>
              <TableHead>{t('list.columns.deal')}</TableHead>
              <TableHead>{t('list.columns.company')}</TableHead>
              <TableHead>{t('list.columns.status')}</TableHead>
              <TableHead className="text-right">{t('list.columns.total')}</TableHead>
              <TableHead>{t('list.columns.validUntil')}</TableHead>
              <TableHead>{t('list.columns.updatedAt')}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {query.data.data.map((quote) => (
              <TableRow key={quote.id}>
                <TableCell className="tabular-nums">{t('numberVersion', { number: quote.number, version: quote.version })}</TableCell>
                <TableCell>
                  <Link to={quotePath(quote.id)} className="font-medium hover:underline">
                    {quote.title}
                  </Link>
                </TableCell>
                <TableCell className="max-w-56 truncate">
                  <Link to={`/deals/${encodeURIComponent(quote.deal_id)}`} className="text-muted-foreground hover:underline">
                    {quote.deal_title ?? '—'}
                  </Link>
                </TableCell>
                <TableCell className="max-w-48 truncate">
                  {quote.company_id ? (
                    <Link to={`/organizations/${encodeURIComponent(quote.company_id)}`} className="text-muted-foreground hover:underline">
                      {quote.company_name ?? '—'}
                    </Link>
                  ) : (
                    '—'
                  )}
                </TableCell>
                <TableCell>
                  <QuoteStatusBadge status={quote.status} />
                </TableCell>
                <TableCell className="text-right tabular-nums">{formatCurrency(quote.total, formatting, quote.currency)}</TableCell>
                <TableCell>{quote.valid_until ? formatDate(quote.valid_until, formatting) : '—'}</TableCell>
                <TableCell>{formatDateTime(quote.updated_at, formatting)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
      <Pagination page={page} perPage={QUOTES_PER_PAGE} total={total} onPageChange={setPage} />
    </Page>
  )
}
