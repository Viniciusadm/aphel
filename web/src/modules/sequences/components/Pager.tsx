import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'

export interface PagerProps {
  page: number
  perPage: number
  total: number
  onPageChange: (page: number) => void
}

export function Pager({ page, perPage, total, onPageChange }: PagerProps) {
  const { t } = useTranslation('sequences')
  const pages = Math.max(1, Math.ceil(total / perPage))
  if (pages <= 1) return null
  return (
    <nav aria-label={t('pager.label')} className="flex flex-wrap items-center justify-between gap-2 text-sm text-muted-foreground">
      <span>{t('pager.summary', { page, pages, total })}</span>
      <div className="flex gap-2">
        <Button variant="outline" size="sm" onClick={() => onPageChange(page - 1)} disabled={page <= 1}>
          {t('pager.previous')}
        </Button>
        <Button variant="outline" size="sm" onClick={() => onPageChange(page + 1)} disabled={page >= pages}>
          {t('pager.next')}
        </Button>
      </div>
    </nav>
  )
}
