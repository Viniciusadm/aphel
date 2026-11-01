import { ChevronLeft, ChevronRight } from 'lucide-react'
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

  if (pages < 2) return null

  return (
    <nav
      data-slot="pagination"
      aria-label={labels.paginationPage(page, pages)}
      className={cn('grid w-full min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2 py-2 md:ml-auto md:w-fit md:gap-4', className)}
    >
      <Button variant="outline" className="gap-1 px-2 text-xs md:px-3 md:text-sm" aria-label={labels.paginationPrevious} disabled={page <= 1} onClick={() => onPageChange(page - 1)}>
        <ChevronLeft aria-hidden />{labels.paginationPreviousShort ?? labels.paginationPrevious}
      </Button>
      <span className="min-w-0 text-center text-sm text-muted-foreground tabular-nums" aria-live="polite">{labels.paginationPage(page, pages)}</span>
      <Button variant="outline" className="gap-1 px-2 text-xs md:px-3 md:text-sm" aria-label={labels.paginationNext} disabled={page >= pages} onClick={() => onPageChange(page + 1)}>
        {labels.paginationNextShort ?? labels.paginationNext}<ChevronRight aria-hidden />
      </Button>
    </nav>
  )
}
