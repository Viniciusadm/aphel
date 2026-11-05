import { ChevronDown, Inbox, PartyPopper, Undo2, X } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import {useActivityInbox, type InboxGroup, type InboxItem, type InboxQuery} from '@/sdk'
import { NativeSelect, useRecordFormatting } from '@/sdk'
import {useViewTeams} from '@/sdk'
import { cn } from '@/lib/utils'
import { InboxItemRow } from './InboxItemRow'
import { postponedDueAt } from './lib/dates'
import { INBOX_OWNERS, INBOX_SORTS, buildInboxParams, parseInboxQuery } from './lib/params'
import { ReassignDialog } from './ReassignDialog'
import { useInboxActions } from './useInboxActions'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'
import { Field } from '@/components/ui/field'

const COLLAPSIBLE_GROUPS = new Set(['upcoming', 'no_due'])

export function InboxPage() {
  const { t } = useTranslation('tasks', { keyPrefix: 'inbox' })
  const { me } = useSession()
  const formatting = useRecordFormatting()
  const [searchParams, setSearchParams] = useSearchParams()
  const query = parseInboxQuery(searchParams)
  const inbox = useActivityInbox(query)
  const actions = useInboxActions(query)
  const teams = useViewTeams(query.owner === 'team')
  const [reassigning, setReassigning] = useState<InboxItem | null>(null)
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({})
  const ids = { owner: useId(), team: useId(), sort: useId() }

  const setQuery = (patch: Partial<InboxQuery>) =>
    setSearchParams((current) => buildInboxParams({ ...query, ...patch }, current), { replace: true })

  const postpone = (item: InboxItem, days: number) => {
    const dueAt = postponedDueAt(item.due_at, days, formatting.timezone)
    if (!dueAt) return
    void actions.update(
      item,
      { due_at: dueAt },
      t(days === 1 ? 'toast.postponedDay' : 'toast.postponedWeek', { title: item.title }),
      { due_at: item.due_at ?? null },
    )
  }

  const reassign = (item: InboxItem, ownerId: string) => {
    setReassigning(null)
    void actions.update(item, { owner_id: ownerId }, t('toast.reassigned', { title: item.title }), {
      owner_id: item.owner_id ?? me.user.id,
    })
  }

  const groups: InboxGroup[] = inbox.data?.groups ?? []
  const count = groups.reduce((total, group) => total + group.items.length, 0)
  const showAmounts = inbox.data?.deal_amounts_visible ?? false

  let body
  if (inbox.isPending) {
    body = (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-5 w-32" />
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-14 w-full" />
      </div>
    )
  } else if (inbox.isError) {
    body = <ErrorState onRetry={() => void inbox.refetch()} />
  } else if (count === 0) {
    body = (
      <EmptyState
        icon={query.owner === 'me' ? PartyPopper : Inbox}
        title={t(`empty.${query.owner}.title`)}
        description={t(`empty.${query.owner}.description`)}
      />
    )
  } else {
    body = (
      <div className={cn('grid gap-6', inbox.isFetching && 'opacity-90')}>
        {inbox.data.truncated && (
          <p className="text-sm text-muted-foreground">
            {t('truncated', { shown: count, total: inbox.data.total })}
          </p>
        )}
        {groups
          .filter((group) => group.items.length > 0)
          .map((group) => {
            const collapsible = COLLAPSIBLE_GROUPS.has(group.key)
            const isCollapsed = collapsible && (collapsed[group.key] ?? false)
            const headingId = `inbox-${group.key}`
            return (
              <section key={group.key} className="grid gap-2" aria-labelledby={headingId}>
                <h2 id={headingId} className="text-sm font-semibold text-muted-foreground">
                  {collapsible ? (
                    <button
                      type="button"
                      className="inline-flex items-center gap-1 hover:text-foreground"
                      aria-expanded={!isCollapsed}
                      onClick={() => setCollapsed((current) => ({ ...current, [group.key]: !isCollapsed }))}
                    >
                      <ChevronDown
                        className={cn('size-4 transition-transform', isCollapsed && '-rotate-90')}
                        aria-hidden="true"
                      />
                      {t(`groups.${group.key}`)} <span className="font-normal">({group.items.length})</span>
                    </button>
                  ) : (
                    <>
                      {t(`groups.${group.key}`)} <span className="font-normal">({group.items.length})</span>
                    </>
                  )}
                </h2>
                {!isCollapsed && (
                  <ul className="grid gap-2">
                    {group.items.map((item) => (
                      <InboxItemRow
                        key={item.id}
                        item={item}
                        group={group.key}
                        showAmounts={showAmounts}
                        pending={actions.isPending(item.id)}
                        onComplete={(target) => void actions.complete(target)}
                        onPostpone={postpone}
                        onReassign={setReassigning}
                      />
                    ))}
                  </ul>
                )}
              </section>
            )
          })}
      </div>
    )
  }

  return (
    <Page>
      <PageHeader title={t('page.title')} description={t('page.description')} />
      <div className="flex min-w-0 flex-wrap items-end gap-3">
        <div className="grid gap-1.5">
          <span id={ids.owner} className="text-xs font-medium text-muted-foreground">
            {t('filters.owner')}
          </span>
          <div role="radiogroup" aria-labelledby={ids.owner} className="inline-flex rounded-md border p-0.5">
            {INBOX_OWNERS.map((owner) => (
              <button
                key={owner}
                type="button"
                role="radio"
                aria-checked={query.owner === owner}
                className={cn(
                  'rounded px-3 py-1 text-sm font-medium transition-colors',
                  query.owner === owner
                    ? 'bg-primary-solid text-primary-foreground'
                    : 'text-muted-foreground hover:text-foreground',
                )}
                onClick={() => setQuery({ owner, teamId: undefined })}
              >
                {t(`owner.${owner}`)}
              </button>
            ))}
          </div>
        </div>
        {query.owner === 'team' && (
          <Field label={t('filters.team')} className="min-w-0">
            {(fieldControl) => <NativeSelect
              {...fieldControl}
              className="w-full sm:w-48"
              value={query.teamId ?? ''}
              onChange={(event) => setQuery({ teamId: event.target.value || undefined })}
            >
              <option value="">{t('filters.myTeams')}</option>
              {(teams.data ?? []).map((team) => (
                <option key={team.id} value={team.id}>
                  {team.name}
                </option>
              ))}
            </NativeSelect>}
          </Field>
        )}
        <Field label={t('filters.sort')} className="min-w-0">
          {(fieldControl) => <NativeSelect
            {...fieldControl}
            className="w-full sm:w-56"
            value={query.sort}
            onChange={(event) => setQuery({ sort: event.target.value as InboxQuery['sort'] })}
          >
            {INBOX_SORTS.map((sort) => (
              <option key={sort} value={sort}>
                {t(`sort.${sort}`)}
              </option>
            ))}
          </NativeSelect>}
        </Field>
      </div>
      {actions.undo && (
        <div
          role="status"
          className="flex min-w-0 items-center gap-2 rounded-md border bg-muted/50 px-3 py-2 text-sm"
        >
          <span className="min-w-0 flex-1 truncate">{actions.undo.message}</span>
          <Button variant="outline" size="sm" onClick={() => void actions.runUndo()}>
            <Undo2 aria-hidden="true" />
            {t('undo.action')}
          </Button>
          <Button variant="ghost" size="icon" aria-label={t('undo.dismiss')} onClick={actions.dismissUndo}>
            <X aria-hidden="true" />
          </Button>
        </div>
      )}
      {body}
      <ReassignDialog item={reassigning} onOpenChange={(open) => !open && setReassigning(null)} onConfirm={reassign} />
    </Page>
  )
}
