import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'
import { EmptyState, type EmptyStateProps } from './empty-state'
import { ErrorState } from './error-state'
import { Skeleton } from './skeleton'

export interface QueryLike<T> {
  isPending: boolean
  isError: boolean
  data: T | undefined
  refetch: () => unknown
}

export interface QueryStateProps<T> {
  query: QueryLike<T>
  empty?: Omit<EmptyStateProps, 'className'> & { className?: string }
  isEmpty?: (data: T) => boolean
  skeletonClassName?: string
  children: (data: T) => ReactNode
}

export function QueryState<T>({ query, empty, isEmpty, skeletonClassName, children }: QueryStateProps<T>) {
  if (query.isPending) return <Skeleton aria-busy className={cn('h-40 w-full', skeletonClassName)} />
  if (query.data === undefined) return <ErrorState onRetry={() => void query.refetch()} />
  const isEmptyResult = isEmpty ? isEmpty(query.data) : Array.isArray(query.data) && query.data.length === 0
  if (isEmptyResult && empty) return <EmptyState {...empty} />
  return <>{children(query.data)}</>
}
