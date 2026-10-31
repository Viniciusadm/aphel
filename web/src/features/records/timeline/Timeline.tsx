import { OptionGroup } from '@/components/ui/option-group'
import { useInfiniteQuery } from '@tanstack/react-query'
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

export function Timeline({ queryKey, fetchPage, kindOptions = [], autoLoad = true, now, className }: TimelineProps) {
  const { t } = useTranslation('records-detail')
  const formatting = useRecordFormatting()
  const labelFor = useDayLabel()
  const [kinds, setKinds] = useState<string[]>([])
  const sentinelRef = useRef<HTMLDivElement>(null)
  const [mountedAt] = useState(() => new Date())

  const query = useInfiniteQuery({
    queryKey: [...queryKey, 'timeline', { kinds }],
    queryFn: ({ pageParam }) => fetchPage({ cursor: pageParam, kinds }),
    initialPageParam: null as string | null,
    getNextPageParam: (lastPage) => lastPage.meta?.next_cursor ?? undefined,
  })
  const { hasNextPage, isFetchingNextPage, fetchNextPage } = query

  const groups = useMemo(() => {
    const items = query.data?.pages.flatMap((page) => page.data) ?? []
    return groupTimelineByDay(filterTimelineItems(items, kinds), formatting.timezone)
  }, [query.data, kinds, formatting.timezone])

  useEffect(() => {
    const sentinel = sentinelRef.current
    if (!autoLoad || !sentinel || !hasNextPage || typeof IntersectionObserver === 'undefined') return
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting) && !isFetchingNextPage) void fetchNextPage()
    })
    observer.observe(sentinel)
    return () => observer.disconnect()
  }, [autoLoad, hasNextPage, isFetchingNextPage, fetchNextPage])

  const ALL_KINDS = '__all__'
  const selectKinds = (next: string[]) =>
    setKinds(next.includes(ALL_KINDS) && kinds.length > 0 ? [] : next.filter((item) => item !== ALL_KINDS))

  const reference = now ?? mountedAt

  return (
    <section aria-label={t('timeline.label')} className={cn('grid gap-4', className)}>
      {kindOptions.length > 0 && (
        <OptionGroup
          multiple
          size="sm"
          aria-label={t('timeline.filterLabel')}
          className="-mx-1 flex-nowrap overflow-x-auto px-1 pb-1"
          value={kinds.length === 0 ? [ALL_KINDS] : kinds}
          onValueChange={selectKinds}
          options={[
            { value: ALL_KINDS, label: t('timeline.all') },
            ...kindOptions.map((option) => ({ value: option.kind, label: option.label })),
          ]}
        />
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

      {hasNextPage && (
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
