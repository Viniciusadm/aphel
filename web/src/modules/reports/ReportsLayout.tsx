import { useTranslation } from 'react-i18next'
import { Link, Outlet, useMatch } from 'react-router'
import { BarChart3, ArrowUpRight } from 'lucide-react'
import { Slot } from '@/kernel/modules/Slot'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function ReportsLayout() {
  const { t } = useTranslation('reports')
  const index = useMatch('/reports')
  return <Page>
    <PageHeader title={t('title')} description={t('description')} back={!index ? { to: '/reports', label: t('library.title') } : undefined} />
    <Outlet />
  </Page>
}

export function ReportsIndex() {
  const { t } = useTranslation('reports')
  return <nav aria-label={t('library.title')} className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
    <Link to="saved" className="grid gap-4 rounded-xl border bg-card p-5 hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
      <BarChart3 className="size-6 text-primary" aria-hidden />
      <div className="flex items-center justify-between gap-3"><h2 className="text-base font-semibold">{t('tabs.saved')}</h2><ArrowUpRight className="size-4" aria-hidden /></div>
      <p className="text-sm text-muted-foreground">{t('library.saved')}</p>
    </Link>
    <Slot name="reports.library" props={{}} />
  </nav>
}
