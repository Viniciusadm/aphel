import { ListOrdered, Plus, Search } from 'lucide-react'
import { useEffect, useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useSequences } from './api'
import { SequenceStatusBadge } from './components/Badges'
import { Pager } from './components/Pager'
import {
  SEQUENCES_MANAGE_PERMISSION,
  SEQUENCES_PATH,
  SEQUENCES_PER_PAGE,
  SEQUENCES_READ_PERMISSION,
  sequencePath,
} from './constants'

type ActiveFilter = '' | 'active' | 'inactive'

function useDebounced<T>(value: T, delay = 300): T {
  const [debounced, setDebounced] = useState(value)
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), delay)
    return () => clearTimeout(timer)
  }, [value, delay])
  return debounced
}

export function SequencesPage() {
  const { t } = useTranslation('sequences')
  const { hasPermission } = useSession()
  if (!hasPermission(SEQUENCES_READ_PERMISSION) && !hasPermission(SEQUENCES_MANAGE_PERMISSION)) {
    return <EmptyState title={t('page.noAccessTitle')} description={t('page.noAccessDescription')} />
  }
  return <SequencesList />
}

function SequencesList() {
  const { t } = useTranslation('sequences')
  const { hasPermission } = useSession()
  const searchId = useId()
  const activeId = useId()
  const canManage = hasPermission(SEQUENCES_MANAGE_PERMISSION)
  const [search, setSearch] = useState('')
  const [active, setActive] = useState<ActiveFilter>('')
  const [page, setPage] = useState(1)
  const term = useDebounced(search.trim())
  const sequences = useSequences({
    page,
    search: term || undefined,
    active: active === '' ? undefined : active === 'active',
  })

  const newButton = canManage ? (
    <Button asChild>
      <Link to={`${SEQUENCES_PATH}/new`}>
        <Plus aria-hidden="true" />
        {t('page.new')}
      </Link>
    </Button>
  ) : null

  let body
  if (sequences.isPending) {
    body = (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-14 w-full" />
      </div>
    )
  } else if (sequences.isError) {
    body = <ErrorState onRetry={() => void sequences.refetch()} />
  } else if (sequences.data.data.length === 0) {
    const filtered = Boolean(term || active)
    body = (
      <EmptyState
        icon={ListOrdered}
        title={filtered ? t('page.noResultsTitle') : t('page.emptyTitle')}
        description={filtered ? t('page.noResultsDescription') : t('page.emptyDescription')}
        action={filtered ? undefined : newButton}
      />
    )
  } else {
    body = (
      <>
        <div className="relative overflow-x-auto rounded-lg border">
          <table className="w-full text-sm">
            <thead className="bg-muted/50 text-left text-muted-foreground">
              <tr>
                <th scope="col" className="px-4 py-2 font-medium">
                  {t('columns.name')}
                </th>
                <th scope="col" className="hidden px-4 py-2 font-medium sm:table-cell">
                  {t('columns.steps')}
                </th>
                <th scope="col" className="px-4 py-2 font-medium">
                  {t('columns.active')}
                </th>
                <th scope="col" className="hidden px-4 py-2 font-medium md:table-cell">
                  {t('columns.completed')}
                </th>
                <th scope="col" className="hidden px-4 py-2 font-medium md:table-cell">
                  {t('columns.exited')}
                </th>
                <th scope="col" className="px-4 py-2 font-medium">
                  {t('columns.status')}
                </th>
              </tr>
            </thead>
            <tbody>
              {sequences.data.data.map((sequence) => (
                <tr key={sequence.id} className="border-t align-top">
                  <td className="px-4 py-3">
                    <Link to={sequencePath(sequence.id)} className="font-medium hover:underline">
                      {sequence.name}
                    </Link>
                    {sequence.description && (
                      <div className="line-clamp-1 text-xs text-muted-foreground">{sequence.description}</div>
                    )}
                  </td>
                  <td className="hidden px-4 py-3 tabular-nums sm:table-cell">{sequence.steps.length}</td>
                  <td className="px-4 py-3 tabular-nums">
                    {sequence.enrollments.active}
                    {sequence.enrollments.paused > 0 && (
                      <span className="ml-1 text-xs text-muted-foreground">
                        {t('page.pausedCount', { count: sequence.enrollments.paused })}
                      </span>
                    )}
                  </td>
                  <td className="hidden px-4 py-3 tabular-nums md:table-cell">{sequence.enrollments.completed}</td>
                  <td className="hidden px-4 py-3 tabular-nums md:table-cell">{sequence.enrollments.exited}</td>
                  <td className="px-4 py-3">
                    <SequenceStatusBadge active={sequence.is_active} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <Pager page={page} perPage={SEQUENCES_PER_PAGE} total={sequences.data.meta.total} onPageChange={setPage} />
      </>
    )
  }

  return (
    <div className="flex w-full flex-col gap-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="grid gap-1">
          <h1 className="text-xl font-semibold">{t('page.title')}</h1>
          <p className="text-sm text-muted-foreground">{t('page.description')}</p>
        </div>
        {newButton}
      </div>
      <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_12rem]">
        <div className="grid gap-1.5">
          <Label htmlFor={searchId} className="sr-only">
            {t('page.search')}
          </Label>
          <div className="relative">
            <Search
              className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
              aria-hidden="true"
            />
            <Input
              id={searchId}
              value={search}
              placeholder={t('page.search')}
              onChange={(event) => {
                setSearch(event.target.value)
                setPage(1)
              }}
              className="pl-9"
            />
          </div>
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor={activeId} className="sr-only">
            {t('page.statusFilter')}
          </Label>
          <NativeSelect
            id={activeId}
            value={active}
            onChange={(event) => {
              setActive(event.target.value as ActiveFilter)
              setPage(1)
            }}
          >
            <option value="">{t('page.allStatuses')}</option>
            <option value="active">{t('status.active')}</option>
            <option value="inactive">{t('status.inactive')}</option>
          </NativeSelect>
        </div>
      </div>
      {body}
    </div>
  )
}
