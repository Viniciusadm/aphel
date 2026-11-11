import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { History } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Card } from '@/components/ui/card'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Pagination } from '@/components/ui/pagination'
import { QueryState } from '@/components/ui/query-state'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { consoleApi, consoleKeys, type AuditEntry, type AuditPage as AuditResult } from '../api'
import { consoleText } from '../errors'
import { formatDateTime } from '../format'

const DETAIL_KEYS = ['slug', 'email', 'host', 'hosts', 'export', 'exported_to'] as const

function actionLabel(action: string): string {
  return consoleText(`audit.actions.${action}`) ?? action
}

function detailSummary(entry: AuditEntry): string {
  const details = (entry.details ?? {}) as Record<string, unknown>
  return DETAIL_KEYS.map((key) => details[key])
    .filter((value) => value !== null && value !== undefined && value !== '')
    .map((value) => (Array.isArray(value) ? value.join(', ') : String(value)))
    .join(' · ')
}

export function AuditPage() {
  const { t } = useTranslation('console')
  const [page, setPage] = useState(1)
  const audit = useQuery({
    queryKey: consoleKeys.audit(page),
    queryFn: () => consoleApi.audit(page),
    placeholderData: keepPreviousData,
  })

  return (
    <Page>
      <PageHeader title={t('audit.title')} description={t('audit.description')} />
      <QueryState
        query={audit}
        isEmpty={(result: AuditResult) => result.data.length === 0}
        empty={{ icon: History, title: t('audit.empty') }}
      >
        {(result: AuditResult) => (
          <div className="grid gap-4">
            <Card className="overflow-x-auto p-0">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>{t('audit.when')}</TableHead>
                    <TableHead>{t('audit.who')}</TableHead>
                    <TableHead>{t('audit.what')}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {result.data.map((entry) => (
                    <TableRow key={entry.id}>
                      <TableCell className="whitespace-nowrap text-sm text-muted-foreground">{formatDateTime(entry.occurred_at)}</TableCell>
                      <TableCell className="text-sm">{entry.operator_email ?? t('audit.commandLine')}</TableCell>
                      <TableCell className="text-sm">
                        <div>{actionLabel(entry.action)}</div>
                        <div className="text-xs text-muted-foreground">{detailSummary(entry)}</div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </Card>
            <Pagination page={result.meta.page} perPage={result.meta.per_page} total={result.meta.total} onPageChange={setPage} />
          </div>
        )}
      </QueryState>
    </Page>
  )
}
