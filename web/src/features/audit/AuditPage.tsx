import { pluginDisplayName } from '@/kernel/modules/plugins'
import { History } from 'lucide-react'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { isApiError } from '@/api'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { ListSkeleton } from '@/features/settings/components/ListSkeleton'
import { Pagination } from '@/components/ui/pagination'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { AUDIT_READ } from '@/features/settings/lib/permissions'
import { AUDIT_PER_PAGE, useAuditLogs, type AuditActor, type AuditLog } from './api'
import { AuditFiltersForm } from './AuditFiltersForm'
import { ChangeList } from './ChangeList'
import { filtersFromSearch, hasFilters, pageFromSearch, toSearch, type AuditFilters } from './lib/filters'
import { recordTypeLabel } from './lib/labels'
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Page } from '@/components/ui/page'

export function AuditPage() {
  return (
    <RequirePermission permission={AUDIT_READ}>
      <AuditSection />
    </RequirePermission>
  )
}

function AuditSection() {
  const { t } = useTranslation('audit')
  const [searchParams, setSearchParams] = useSearchParams()
  const search = searchParams.toString()
  const filters = useMemo(() => filtersFromSearch(new URLSearchParams(search)), [search])
  const page = pageFromSearch(searchParams)
  const logs = useAuditLogs(filters, page)

  const apply = (next: AuditFilters, nextPage = 1) => setSearchParams(toSearch(next, nextPage))
  const filterByActor = (actorId: string) => apply({ ...filters, actor: actorId })

  const invalidFilters = logs.isError && isApiError(logs.error) && logs.error.status === 422

  return (
    <Page>
      <PageHeader title={t('title')} description={t('description')} />
      <AuditFiltersForm key={toSearch(filters).toString()} filters={filters} onApply={(next) => apply(next)} />
      {logs.isPending ? (
        <ListSkeleton />
      ) : logs.isError ? (
        <ErrorState
          title={invalidFilters ? t('errors.invalidFilters') : undefined}
          onRetry={invalidFilters ? undefined : () => void logs.refetch()}
        />
      ) : logs.data.data.length === 0 ? (
        <EmptyState
          icon={History}
          title={hasFilters(filters) ? t('empty.filtered') : t('empty.title')}
          description={hasFilters(filters) ? undefined : t('empty.description')}
        />
      ) : (
        <>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead scope="col" className="whitespace-nowrap">
                  {t('columns.occurredAt')}
                </TableHead>
                <TableHead scope="col">
                  {t('columns.actor')}
                </TableHead>
                <TableHead scope="col">
                  {t('columns.action')}
                </TableHead>
                <TableHead scope="col" className="hidden md:table-cell">
                  {t('columns.record')}
                </TableHead>
                <TableHead scope="col">
                  {t('columns.changes')}
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {logs.data.data.map((log) => (
                <AuditRow key={log.id} log={log} onFilterActor={filterByActor} />
              ))}
            </TableBody>
          </Table>
          <Pagination
            page={page}
            perPage={AUDIT_PER_PAGE}
            total={logs.data.meta.total}
            onPageChange={(next) => apply(filters, next)}
          />
        </>
      )}
    </Page>
  )
}

function useDateTimeFormat() {
  const { i18n } = useTranslation()
  return useMemo(
    () => new Intl.DateTimeFormat(i18n.language, { dateStyle: 'short', timeStyle: 'medium' }),
    [i18n.language],
  )
}

function AuditRow({ log, onFilterActor }: { log: AuditLog; onFilterActor: (actorId: string) => void }) {
  const { t } = useTranslation('audit')
  const format = useDateTimeFormat()
  const recordLabelKey = log.record_type ? recordTypeLabel(log.record_type) : undefined

  return (
    <tr className="border-t align-top">
      <td className="px-4 py-3 whitespace-nowrap text-muted-foreground">
        <time dateTime={log.occurred_at}>{format.format(new Date(log.occurred_at))}</time>
      </td>
      <td className="px-4 py-3">
        <ActorCell actor={log.actor} onFilterActor={onFilterActor} />
      </td>
      <td className="px-4 py-3">
        <code className="rounded bg-muted px-1.5 py-0.5 text-xs">{log.action}</code>
      </td>
      <td className="hidden px-4 py-3 md:table-cell">
        {log.record_type ? (
          <div className="grid gap-0.5">
            <span>{recordLabelKey ? t(recordLabelKey) : log.record_type}</span>
            {log.record_id && (
              <span className="font-mono text-xs break-all text-muted-foreground">{log.record_id}</span>
            )}
          </div>
        ) : (
          <span className="text-muted-foreground">{t('record.none')}</span>
        )}
      </td>
      <td className="px-4 py-3">
        <ChangeList changes={log.changes} />
      </td>
    </tr>
  )
}

function ActorCell({ actor, onFilterActor }: { actor: AuditActor; onFilterActor: (actorId: string) => void }) {
  const { t } = useTranslation('audit')
  const label =
    actor.type === 'user'
      ? (actor.name ?? actor.email ?? actor.id ?? t('actor.unknownUser'))
      : actor.type === 'system'
        ? t('actor.system')
        : actor.type === 'api_token'
          ? t('actor.apiToken')
          : actor.type === 'plugin'
            ? t('actor.plugin', { key: pluginDisplayName(actor.id ?? '') })
            : t('actor.automation')

  return (
    <div className="grid gap-0.5">
      {actor.id ? (
        <Button
          variant="link"
          className="h-auto justify-start p-0 text-left text-foreground"
          title={t('actor.filterBy')}
          onClick={() => actor.id && onFilterActor(actor.id)}
        >
          {label}
        </Button>
      ) : (
        <span>{label}</span>
      )}
      {actor.type === 'user' && actor.email && actor.name && (
        <span className="text-xs text-muted-foreground">{actor.email}</span>
      )}
    </div>
  )
}
