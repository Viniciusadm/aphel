import { ListOrdered, Plus } from 'lucide-react'
import { useEffect, useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { SegmentedControl } from '@/components/ui/segmented-control'
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
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Page } from '@/components/ui/page'
import { Field } from '@/components/ui/field'
import { SearchInput } from '@/components/ui/search-input'

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
    <Button asChild size="sm">
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
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">
                {t('columns.name')}
              </TableHead>
              <TableHead scope="col" className="hidden sm:table-cell">
                {t('columns.steps')}
              </TableHead>
              <TableHead scope="col">
                {t('columns.active')}
              </TableHead>
              <TableHead scope="col" className="hidden md:table-cell">
                {t('columns.completed')}
              </TableHead>
              <TableHead scope="col" className="hidden md:table-cell">
                {t('columns.exited')}
              </TableHead>
              <TableHead scope="col">
                {t('columns.status')}
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {sequences.data.data.map((sequence) => (
              <TableRow key={sequence.id} className="align-top">
                <TableCell>
                  <Link to={sequencePath(sequence.id)} className="font-medium hover:underline">
                    {sequence.name}
                  </Link>
                  {sequence.description && (
                    <div className="line-clamp-1 text-xs text-muted-foreground">{sequence.description}</div>
                  )}
                </TableCell>
                <TableCell className="hidden tabular-nums sm:table-cell">{sequence.steps.length}</TableCell>
                <TableCell className="tabular-nums">
                  {sequence.enrollments.active}
                  {sequence.enrollments.paused > 0 && (
                    <span className="ml-1 text-xs text-muted-foreground">
                      {t('page.pausedCount', { count: sequence.enrollments.paused })}
                    </span>
                  )}
                </TableCell>
                <TableCell className="hidden tabular-nums md:table-cell">{sequence.enrollments.completed}</TableCell>
                <TableCell className="hidden tabular-nums md:table-cell">{sequence.enrollments.exited}</TableCell>
                <TableCell>
                  <SequenceStatusBadge active={sequence.is_active} />
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
        <Pager page={page} perPage={SEQUENCES_PER_PAGE} total={sequences.data.meta.total} onPageChange={setPage} />
      </>
    )
  }

  return (
    <Page>
<PageHeader title={t('page.title')} description={t('page.description')} actions={<>
        {newButton}
      
      </>} />
      <div className="flex flex-wrap items-center gap-3">
        <Field id={searchId} label={t('page.search')} hideLabel className="min-w-56 flex-1 sm:max-w-sm">
          <SearchInput
            id={searchId}
            value={search}
            placeholder={t('page.search')}
            onValueChange={(value) => {
              setSearch(value)
              setPage(1)
            }}
          />
        </Field>
        <SegmentedControl
          aria-label={t('page.statusFilter')}
          size="sm"
          value={active}
          onValueChange={(value) => {
            setActive(value as ActiveFilter)
            setPage(1)
          }}
          options={[
            { value: '', label: t('page.allStatuses') },
            { value: 'active', label: t('page.filterActive') },
            { value: 'inactive', label: t('page.filterInactive') },
          ]}
        />
      </div>
      {body}
    </Page>
  )
}
