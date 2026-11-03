import { TimelineFilters } from './TimelineFilters'
import { useInfiniteQuery, useQuery } from '@tanstack/react-query'
import { LoaderCircle } from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'
import { useRecordFormatting } from '../formatting'
import { filterTimelineItems, groupTimelineByDay, timelineDayLabel, type TimelineDayLabel } from './grouping'
import { TimelineEntry } from './TimelineEntry'
import type { TimelineFetchPage, TimelineKindOption } from './types'

export interface TimelineProps {
  queryKey: readonly unknown[]
  fetchPage: TimelineFetchPage
  kindOptions?: TimelineKindOption[]
  fetchKindOptions?: () => Promise<TimelineKindOption[]>
  limit?: number
  autoLoad?: boolean
  now?: Date
  className?: string
}

function useDayLabel() {
  const { t } = useTranslation('records-detail')
  return (label: TimelineDayLabel) => {
    if (label.kind === 'today') return t('timeline.today')
    if (label.kind === 'yesterday') return t('timeline.yesterday')
    return label.text
  }
}

function TimelineSkeleton() {
  return (
    <div className="grid gap-4" aria-hidden="true">
      {[0, 1, 2].map((index) => (
        <div key={index} className="flex gap-3">
          <Skeleton className="size-8 rounded-full" />
          <div className="grid flex-1 gap-2">
            <Skeleton className="h-3 w-32" />
            <Skeleton className="h-4 w-3/4" />
          </div>
        </div>
      ))}
    </div>
  )
}

const EMPTY_KIND_OPTIONS: TimelineKindOption[] = []

export function Timeline({ queryKey, fetchPage, kindOptions: fixedOptions = EMPTY_KIND_OPTIONS, fetchKindOptions, autoLoad = true, limit, now, className }: TimelineProps) {
  const { t } = useTranslation('records-detail')
  const formatting = useRecordFormatting()
  const labelFor = useDayLabel()
  const [kinds, setKinds] = useState<string[]>([])
  const sentinelRef = useRef<HTMLDivElement>(null)
  const [mountedAt] = useState(() => new Date())

  const kindQuery = useQuery({ queryKey: [...queryKey, 'timeline-kinds', formatting.locale], queryFn: fetchKindOptions ?? (() => Promise.resolve(fixedOptions)), enabled: Boolean(fetchKindOptions) })
  const kindOptions = kindQuery.data ?? fixedOptions
  const selectedKinds = useMemo(() => fetchKindOptions && kindQuery.data ? kinds.filter((key) => kindOptions.some((option) => option.kind === key)) : kinds, [fetchKindOptions, kindQuery.data, kinds, kindOptions])

  const query = useInfiniteQuery({
    queryKey: [...queryKey, 'timeline', { kinds: selectedKinds }],
    queryFn: ({ pageParam }) => fetchPage({ cursor: pageParam, kinds: selectedKinds }),
    initialPageParam: null as string | null,
    getNextPageParam: (lastPage) => lastPage.meta?.next_cursor ?? undefined,
  })
  const { hasNextPage, isFetchingNextPage, fetchNextPage } = query

  const groups = useMemo(() => {
    const items = query.data?.pages.flatMap((page) => page.data) ?? []
    return groupTimelineByDay(filterTimelineItems(items, selectedKinds).slice(0, limit), formatting.timezone)
  }, [query.data, selectedKinds, formatting.timezone, limit])

  useEffect(() => {
    const sentinel = sentinelRef.current
    if (!autoLoad || !sentinel || !hasNextPage || typeof IntersectionObserver === 'undefined') return
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting) && !isFetchingNextPage) void fetchNextPage()
    })
    observer.observe(sentinel)
    return () => observer.disconnect()
  }, [autoLoad, hasNextPage, isFetchingNextPage, fetchNextPage])

  const reference = now ?? mountedAt

  return (
    <section aria-label={t('timeline.label')} className={cn('grid gap-4', className)}>
      {!limit && kindOptions.length > 0 && (
        <TimelineFilters options={kindOptions} value={kinds} onChange={setKinds} />
      )}

      {query.isPending ? (
        <TimelineSkeleton />
      ) : query.isError && !query.data ? (
        <ErrorState title={t('timeline.errorTitle')} onRetry={() => void query.refetch()} />
      ) : groups.length === 0 ? (
        <EmptyState
          title={t('timeline.emptyTitle')}
          description={kinds.length > 0 ? t('timeline.emptyFiltered') : t('timeline.emptyDescription')}
        />
      ) : (
        <div className="grid gap-6">
          {groups.map((group) => {
            const label = labelFor(timelineDayLabel(group.key, reference, formatting.timezone, formatting.locale))
            return (
              <section key={group.key} aria-label={label} className="grid gap-3">
                <h3 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">{label}</h3>
                <ol className="grid">
                  {group.items.map((item, index) => (
                    <TimelineEntry
                      key={item.id}
                      item={item}
                      timeZone={formatting.timezone}
                      locale={formatting.locale}
                      isLast={index === group.items.length - 1}
                    />
                  ))}
                </ol>
              </section>
            )
          })}
        </div>
      )}

      {!limit && hasNextPage && (
        <div ref={sentinelRef} className="flex justify-center">
          <Button variant="outline" size="sm" onClick={() => void fetchNextPage()} disabled={isFetchingNextPage}>
            {isFetchingNextPage && <LoaderCircle className="animate-spin" aria-hidden="true" />}
            {isFetchingNextPage ? t('timeline.loadingMore') : t('timeline.loadMore')}
          </Button>
        </div>
      )}
      {!hasNextPage && groups.length > 0 && (query.data?.pages.length ?? 0) > 1 && (
        <p className="text-center text-xs text-muted-foreground">{t('timeline.end')}</p>
      )}
    </section>
  )
}
