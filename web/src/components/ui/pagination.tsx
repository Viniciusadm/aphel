import { ChevronLeft, ChevronRight } from 'lucide-react'
import { useId } from 'react'
import { cn } from '@/lib/utils'
import { Button } from './button'
import { useUiLabels } from './labels-context'
import { NativeSelect } from './native-select'

const PER_PAGE_OPTIONS = [25, 50, 100] as const

export interface PaginationProps {
  page: number
  perPage: number
  total: number
  onPageChange: (page: number) => void
  onPerPageChange?: (perPage: number) => void
  perPageOptions?: readonly number[]
  className?: string
}

export function Pagination({
  page,
  perPage,
  total,
  onPageChange,
  onPerPageChange,
  perPageOptions = PER_PAGE_OPTIONS,
  className,
}: PaginationProps) {
  const labels = useUiLabels()
  const perPageId = useId()
  const pages = Math.max(1, Math.ceil(total / perPage))
  const from = total === 0 ? 0 : (page - 1) * perPage + 1
  const to = Math.min(total, page * perPage)
  const locale = typeof document === 'undefined' ? undefined : document.documentElement.lang || undefined
  const number = (value: number) => value.toLocaleString(locale)

  return (
    <nav
      data-slot="pagination"
      aria-label={labels.paginationPage(page, pages)}
      className={cn('flex flex-wrap items-center justify-between gap-3 text-sm text-muted-foreground', className)}
    >
      <p aria-live="polite">{labels.paginationSummary(number(from), number(to), number(total))}</p>
      <div className="flex flex-wrap items-center gap-3">
        {onPerPageChange && (
          <span className="flex items-center gap-2">
            <label htmlFor={perPageId}>{labels.paginationPerPage}</label>
            <NativeSelect
              id={perPageId}
              className="w-20"
              value={String(perPage)}
              onChange={(event) => onPerPageChange(Number(event.target.value))}
            >
              {perPageOptions.map((option) => (
                <option key={option} value={option}>
                  {option}
                </option>
              ))}
            </NativeSelect>
          </span>
        )}
        <span className="flex items-center gap-1">
          <Button
            variant="outline"
            size="icon"
            className="size-8"
            aria-label={labels.paginationPrevious}
            disabled={page <= 1}
            onClick={() => onPageChange(page - 1)}
          >
            <ChevronLeft aria-hidden />
          </Button>
          <span className="px-2 tabular-nums">{labels.paginationPage(page, pages)}</span>
          <Button
            variant="outline"
            size="icon"
            className="size-8"
            aria-label={labels.paginationNext}
            disabled={page >= pages}
            onClick={() => onPageChange(page + 1)}
          >
            <ChevronRight aria-hidden />
          </Button>
        </span>
      </div>
    </nav>
  )
}
