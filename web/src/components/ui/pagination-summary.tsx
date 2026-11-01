import { cn } from '@/lib/utils'
import { useUiLabels } from './labels-context'

export interface PaginationSummaryProps {
  page: number
  perPage: number
  total: number
  className?: string
}

export function PaginationSummary({ page, perPage, total, className }: PaginationSummaryProps) {
  const labels = useUiLabels()
  const from = total === 0 ? 0 : (page - 1) * perPage + 1
  const to = Math.min(total, page * perPage)
  const locale = typeof document === 'undefined' ? undefined : document.documentElement.lang || undefined
  const number = (value: number) => value.toLocaleString(locale)
  return <p className={cn('text-xs text-muted-foreground tabular-nums', className)} role="status">{labels.paginationSummary(number(from), number(to), number(total))}</p>
}
