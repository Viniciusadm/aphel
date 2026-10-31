import { Search, SearchX } from 'lucide-react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'
import { SEARCH_MIN_LENGTH, useGlobalSearch } from './api'
import { SearchResultGroups } from './results'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

const SEARCH_PAGE_LIMIT = 20
const SEARCH_DEBOUNCE_MS = 250

export function SearchPage() {
  const { t } = useTranslation('search')
  const [params, setParams] = useSearchParams()
  const query = params.get('q') ?? ''
  const [draft, setDraft] = useState(query)
  const [syncedQuery, setSyncedQuery] = useState(query)

  if (syncedQuery !== query) {
    setSyncedQuery(query)
    if (query.trim() !== draft.trim()) setDraft(query)
  }

  useEffect(() => {
    if (draft.trim() === query.trim()) return
    const timer = setTimeout(() => {
      setParams(
        (current) => {
          const next = new URLSearchParams(current)
          if (draft.trim()) next.set('q', draft.trim())
          else next.delete('q')
          return next
        },
        { replace: true },
      )
    }, SEARCH_DEBOUNCE_MS)
    return () => clearTimeout(timer)
  }, [draft, query, setParams])

  const trimmed = query.trim()
  const search = useGlobalSearch({ q: trimmed, limit: SEARCH_PAGE_LIMIT })

  let body
  if (trimmed.length < SEARCH_MIN_LENGTH) {
    body = <EmptyState icon={Search} title={t('page.promptTitle')} description={t('page.promptDescription')} />
  } else if (search.isPending) {
    body = (
      <div className="grid gap-4" aria-busy="true" aria-label={t('page.loading')}>
        <Skeleton className="h-40 w-full" />
        <Skeleton className="h-28 w-full" />
      </div>
    )
  } else if (search.isError) {
    body = <ErrorState description={t('page.error')} onRetry={() => void search.refetch()} />
  } else if (search.data.groups.length === 0) {
    body = (
      <EmptyState icon={SearchX} title={t('page.emptyTitle')} description={t('page.emptyDescription', { query: trimmed })} />
    )
  } else {
    body = (
      <SearchResultGroups
        groups={search.data.groups}
        className={cn('transition-opacity', search.isFetching && 'opacity-60')}
      />
    )
  }

  return (
    <Page>
      <PageHeader title={t('page.title')} description={t('page.description')} />
      <form
        role="search"
        className="relative"
        onSubmit={(event) => {
          event.preventDefault()
          setParams(draft.trim() ? { q: draft.trim() } : {}, { replace: true })
        }}
      >
        <Search className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" aria-hidden />
        <Input
          type="search"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          placeholder={t('page.placeholder')}
          aria-label={t('page.inputLabel')}
          maxLength={100}
          autoFocus
          className="pl-9"
        />
      </form>
      {trimmed.length >= SEARCH_MIN_LENGTH && search.data && !search.isError && (
        <p className="text-xs text-muted-foreground" aria-live="polite">
          {t('page.summary', { count: search.data.groups.reduce((sum, group) => sum + group.total, 0) })}
        </p>
      )}
      {body}
    </Page>
  )
}
