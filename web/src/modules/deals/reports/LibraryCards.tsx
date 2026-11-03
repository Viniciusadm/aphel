import { BarChart3, ArrowUpRight } from 'lucide-react'
import { Link } from 'react-router'
import { useTranslation } from 'react-i18next'

export function DealReportLibraryCards() {
  const { t } = useTranslation('deals')
  return (['conversion', 'performance'] as const).map((key) => <Link key={key} to={`/deals/reports/${key}`} className="grid gap-4 rounded-xl border bg-card p-5 hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
    <BarChart3 className="size-6 text-primary" aria-hidden />
    <div className="flex items-center justify-between gap-3"><h2 className="text-base font-semibold">{t(`reportLibrary.${key}.title`)}</h2><ArrowUpRight className="size-4" aria-hidden /></div>
    <p className="text-sm text-muted-foreground">{t(`reportLibrary.${key}.description`)}</p>
  </Link>)
}
