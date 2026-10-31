import { FileText } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Skeleton } from '@/components/ui/skeleton'
import { formatCurrency, formatDate, useRecordFormatting } from '@/features/records/formatting'
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
  const pages = Math.max(1, Math.ceil(total / QUOTES_PER_PAGE))

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
        <Select
          value={status || ALL}
          onValueChange={(value) => {
            setStatus(value === ALL ? '' : value)
            setPage(1)
          }}
        >
          <SelectTrigger className="w-48" aria-label={t('list.columns.status')}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={ALL}>{t('list.allStatuses')}</SelectItem>
            {STATUSES.map((value) => (
              <SelectItem key={value} value={value}>
                {t(`status.${value}`)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      {query.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </div>
      ) : query.isError ? (
        <ErrorState title={t('list.loadError')} onRetry={() => void query.refetch()} />
      ) : query.data.data.length === 0 ? (
        <EmptyState icon={FileText} title={t('list.empty')} />
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t('list.columns.number')}</TableHead>
              <TableHead>{t('list.columns.title')}</TableHead>
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
                <TableCell>
                  <QuoteStatusBadge status={quote.status} />
                </TableCell>
                <TableCell className="text-right tabular-nums">{formatCurrency(quote.total, formatting, quote.currency)}</TableCell>
                <TableCell>{quote.valid_until ? formatDate(quote.valid_until, formatting) : '—'}</TableCell>
                <TableCell>{formatDate(quote.updated_at, formatting)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
      {pages > 1 && (
        <div className="flex items-center justify-end gap-2 text-sm">
          <Button size="sm" variant="outline" disabled={page <= 1} onClick={() => setPage(page - 1)}>
            {t('list.previous')}
          </Button>
          <span>{t('list.page', { page, pages })}</span>
          <Button size="sm" variant="outline" disabled={page >= pages} onClick={() => setPage(page + 1)}>
            {t('list.next')}
          </Button>
        </div>
      )}
    </Page>
  )
}
