import { ChevronLeft, ChevronRight } from 'lucide-react'
import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { NativeSelect } from '../filters/NativeSelect'
import { PER_PAGE_OPTIONS } from './query'

export interface RecordPaginationProps {
  page: number
  perPage: number
  total: number
  onPageChange: (page: number) => void
  onPerPageChange?: (perPage: number) => void
  className?: string
}

export function RecordPagination({ page, perPage, total, onPageChange, onPerPageChange, className }: RecordPaginationProps) {
  const { t, i18n } = useTranslation('records-table')
  const perPageId = useId()
  const pages = Math.max(1, Math.ceil(total / perPage))
  const from = total === 0 ? 0 : (page - 1) * perPage + 1
  const to = Math.min(total, page * perPage)
  const number = (value: number) => value.toLocaleString(i18n.language)

  return (
    <nav
      aria-label={t('pagination.page', { page, pages })}
      className={cn('flex flex-wrap items-center justify-between gap-3 text-sm text-muted-foreground', className)}
    >
      <p aria-live="polite">{t('pagination.summary', { from: number(from), to: number(to), total: number(total) })}</p>
      <div className="flex flex-wrap items-center gap-3">
        {onPerPageChange && (
          <span className="flex items-center gap-2">
            <label htmlFor={perPageId}>{t('pagination.perPage')}</label>
            <NativeSelect
              id={perPageId}
              className="w-20"
              value={String(perPage)}
              onChange={(event) => onPerPageChange(Number(event.target.value))}
            >
              {PER_PAGE_OPTIONS.map((option) => (
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
            aria-label={t('pagination.previous')}
            disabled={page <= 1}
            onClick={() => onPageChange(page - 1)}
          >
            <ChevronLeft aria-hidden />
          </Button>
          <span className="px-2 tabular-nums">{t('pagination.page', { page, pages })}</span>
          <Button
            variant="outline"
            size="icon"
            className="size-8"
            aria-label={t('pagination.next')}
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
