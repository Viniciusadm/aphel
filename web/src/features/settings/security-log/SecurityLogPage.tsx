import { SegmentedControl } from '@/components/ui/segmented-control'
import { ShieldCheck } from 'lucide-react'
import { useId, useMemo, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { isApiError } from '@/api'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ListSkeleton } from '@/features/settings/components/ListSkeleton'
import { Pagination } from '@/components/ui/pagination'
import { USERS_MANAGE } from '@/features/settings/lib/permissions'
import { DeviceIcon } from '@/features/settings/security/sessions/DeviceIcon'
import { useDeviceLabel } from '@/features/settings/security/sessions/device-label'
import {
  SECURITY_EVENTS_PER_PAGE,
  SECURITY_EVENT_KINDS,
  useSecurityEvents,
  type SecurityEvent,
  type SecurityEventFilters,
  type SecurityEventKind,
} from './api'
import { useEventDetails } from './details'
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Page } from '@/components/ui/page'

const EMPTY_FILTERS: SecurityEventFilters = { scope: 'mine', kind: '', from: '', to: '' }

function filtersFromSearch(params: URLSearchParams, canSeeOrganization: boolean): SecurityEventFilters {
  const kind = params.get('kind') ?? ''
  return {
    scope: canSeeOrganization && params.get('scope') === 'organization' ? 'organization' : 'mine',
    kind: SECURITY_EVENT_KINDS.includes(kind as SecurityEventKind) ? (kind as SecurityEventKind) : '',
    from: params.get('from') ?? '',
    to: params.get('to') ?? '',
  }
}

function pageFromSearch(params: URLSearchParams): number {
  const page = Number.parseInt(params.get('page') ?? '', 10)
  return Number.isFinite(page) && page >= 1 ? page : 1
}

function toSearch(filters: SecurityEventFilters, page = 1): URLSearchParams {
  const params = new URLSearchParams()
  if (filters.scope === 'organization') params.set('scope', 'organization')
  if (filters.kind) params.set('kind', filters.kind)
  if (filters.from) params.set('from', filters.from)
  if (filters.to) params.set('to', filters.to)
  if (page > 1) params.set('page', String(page))
  return params
}

function hasFilters(filters: SecurityEventFilters): boolean {
  return Boolean(filters.kind || filters.from || filters.to)
}

export function SecurityLogPage() {
  const { t } = useTranslation('security')
  const { hasPermission } = useSession()
  const canSeeOrganization = hasPermission(USERS_MANAGE)
  const [searchParams, setSearchParams] = useSearchParams()
  const search = searchParams.toString()
  const filters = useMemo(
    () => filtersFromSearch(new URLSearchParams(search), canSeeOrganization),
    [search, canSeeOrganization],
  )
  const page = pageFromSearch(searchParams)
  const events = useSecurityEvents(filters, page)
  const apply = (next: SecurityEventFilters, nextPage = 1) => setSearchParams(toSearch(next, nextPage))
  const invalidFilters = events.isError && isApiError(events.error) && events.error.status === 422
  const showUser = filters.scope === 'organization'

  return (
    <Page>
      <PageHeader title={t('log.title')} description={t('log.description')} />
      {canSeeOrganization && (
        <SegmentedControl
          aria-label={t('log.scopeLabel')}
          size="sm"
          className="self-start"
          value={filters.scope}
          onValueChange={(scope) => apply({ ...filters, scope })}
          options={(['mine', 'organization'] as const).map((scope) => ({ value: scope, label: t(`log.scope.${scope}`) }))}
        />
      )}
      <FiltersForm key={toSearch(filters).toString()} filters={filters} onApply={(next) => apply(next)} />
      {events.isPending ? (
        <ListSkeleton />
      ) : events.isError ? (
        <ErrorState
          title={invalidFilters ? t('log.invalidFilters') : undefined}
          onRetry={invalidFilters ? undefined : () => void events.refetch()}
        />
      ) : events.data.data.length === 0 ? (
        <EmptyState
          icon={ShieldCheck}
          title={hasFilters(filters) ? t('log.emptyFiltered') : t('log.empty')}
        />
      ) : (
        <>
          <Table>
            <caption className="sr-only">{t('log.title')}</caption>
            <TableHeader>
              <TableRow>
                <TableHead scope="col" className="whitespace-nowrap">
                  {t('log.columns.occurredAt')}
                </TableHead>
                <TableHead scope="col">
                  {t('log.columns.event')}
                </TableHead>
                {showUser && (
                  <TableHead scope="col">
                    {t('log.columns.user')}
                  </TableHead>
                )}
                <TableHead scope="col" className="hidden md:table-cell">
                  {t('log.columns.device')}
                </TableHead>
                <TableHead scope="col" className="hidden lg:table-cell">
                  {t('log.columns.details')}
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {events.data.data.map((event) => (
                <EventRow key={event.id} event={event} showUser={showUser} />
              ))}
            </TableBody>
          </Table>
          <Pagination
            page={page}
            perPage={SECURITY_EVENTS_PER_PAGE}
            total={events.data.meta.total}
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
    () => new Intl.DateTimeFormat(i18n.language, { dateStyle: 'short', timeStyle: 'short' }),
    [i18n.language],
  )
}

function EventRow({ event, showUser }: { event: SecurityEvent; showUser: boolean }) {
  const { t } = useTranslation('security')
  const format = useDateTimeFormat()
  const deviceLabel = useDeviceLabel()
  const details = useEventDetails()(event)
  const device = deviceLabel(event.device)
  const ip = event.ip ?? t('unknownIp')

  return (
    <tr className="border-t align-top">
      <td className="px-3 py-3 whitespace-nowrap text-muted-foreground">
        <time dateTime={event.occurred_at}>{format.format(new Date(event.occurred_at))}</time>
      </td>
      <td className="px-3 py-3">
        <div className="grid gap-0.5">
          <span className="font-medium">{t(`kinds.${event.kind}`)}</span>
          <span className="text-xs text-muted-foreground md:hidden">
            {device} · {ip}
          </span>
          {details && <span className="text-xs break-words text-muted-foreground lg:hidden">{details}</span>}
        </div>
      </td>
      {showUser && (
        <td className="px-3 py-3">
          {event.user ? (
            <div className="grid gap-0.5">
              <span className="break-words">{event.user.name || event.user.email}</span>
              {event.user.name && (
                <span className="text-xs break-all text-muted-foreground">{event.user.email}</span>
              )}
            </div>
          ) : (
            <span className="text-muted-foreground">{t('log.noUser')}</span>
          )}
        </td>
      )}
      <td className="hidden px-3 py-3 md:table-cell">
        <div className="flex items-start gap-2">
          <DeviceIcon device={event.device} className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
          <div className="grid gap-0.5">
            <span>{device}</span>
            <span className="text-xs text-muted-foreground">{ip}</span>
          </div>
        </div>
      </td>
      <td className="hidden px-3 py-3 lg:table-cell">
        <span className="break-words text-muted-foreground">{details ?? '—'}</span>
      </td>
    </tr>
  )
}

interface FiltersFormProps {
  filters: SecurityEventFilters
  onApply: (filters: SecurityEventFilters) => void
}

function FiltersForm({ filters, onApply }: FiltersFormProps) {
  const { t } = useTranslation('security')
  const id = useId()
  const [draft, setDraft] = useState(filters)

  const submit = (event: FormEvent) => {
    event.preventDefault()
    onApply(draft)
  }

  const clear = () => {
    const cleared = { ...EMPTY_FILTERS, scope: filters.scope }
    setDraft(cleared)
    onApply(cleared)
  }

  return (
    <form
      onSubmit={submit}
      aria-label={t('log.filters.label')}
      className="grid gap-3 rounded-lg border p-4 sm:grid-cols-3"
    >
      <div className="grid gap-2.5">
        <Label htmlFor={`${id}-kind`}>{t('log.filters.kind')}</Label>
        <select
          id={`${id}-kind`}
          className="h-9 rounded-md border border-input bg-background px-3 text-sm"
          value={draft.kind}
          onChange={(event) => setDraft((current) => ({ ...current, kind: event.target.value as SecurityEventKind | '' }))}
        >
          <option value="">{t('log.filters.allKinds')}</option>
          {SECURITY_EVENT_KINDS.map((kind) => (
            <option key={kind} value={kind}>
              {t(`kinds.${kind}`)}
            </option>
          ))}
        </select>
      </div>
      <div className="grid gap-2.5">
        <Label htmlFor={`${id}-from`}>{t('log.filters.from')}</Label>
        <Input
          id={`${id}-from`}
          type="date"
          value={draft.from}
          max={draft.to || undefined}
          onChange={(event) => setDraft((current) => ({ ...current, from: event.target.value }))}
        />
      </div>
      <div className="grid gap-2.5">
        <Label htmlFor={`${id}-to`}>{t('log.filters.to')}</Label>
        <Input
          id={`${id}-to`}
          type="date"
          value={draft.to}
          min={draft.from || undefined}
          onChange={(event) => setDraft((current) => ({ ...current, to: event.target.value }))}
        />
      </div>
      <div className="flex flex-wrap justify-end gap-2 sm:col-span-3">
        <Button type="button" variant="outline" onClick={clear} disabled={!hasFilters(draft) && !hasFilters(filters)}>
          {t('log.filters.clear')}
        </Button>
        <Button type="submit">{t('log.filters.apply')}</Button>
      </div>
    </form>
  )
}
