import { cn } from '@/lib/utils'
import { Button } from './button'
import { useUiLabels } from './labels-context'

export interface PaginationProps {
  page: number
  perPage: number
  total: number
  onPageChange: (page: number) => void
  className?: string
}

export function Pagination({ page, perPage, total, onPageChange, className }: PaginationProps) {
  const labels = useUiLabels()
  const pages = Math.max(1, Math.ceil(total / perPage))
  if (pages <= 1) return null
  return (
    <div
      data-slot="pagination"
      className={cn('flex items-center justify-between gap-2 text-sm text-muted-foreground', className)}
    >
      <span>{labels.paginationSummary(page, pages, total)}</span>
      <div className="flex gap-2">
        <Button variant="outline" size="sm" onClick={() => onPageChange(page - 1)} disabled={page <= 1}>
          {labels.paginationPrevious}
        </Button>
        <Button variant="outline" size="sm" onClick={() => onPageChange(page + 1)} disabled={page >= pages}>
          {labels.paginationNext}
        </Button>
      </div>
    </div>
  )
}
