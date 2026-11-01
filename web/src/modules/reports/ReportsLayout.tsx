import { useTranslation } from 'react-i18next'
import { Link, Outlet, useMatch } from 'react-router'
import { BarChart3, ArrowUpRight } from 'lucide-react'
import { useSession } from '@/app/session/session-context'
import { EmptyState } from '@/components/ui/empty-state'
import { allowedReportTabs } from './tabs'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function ReportsLayout() {
  const { t } = useTranslation('reports')
  const { hasPermission } = useSession()
  const index = useMatch('/reports')
  const tabs = allowedReportTabs(hasPermission)
  return <Page>
    <PageHeader title={t('title')} description={t('description')} back={!index ? { to: '/reports', label: t('library.title') } : undefined} />
    {tabs.length === 0 ? <EmptyState title={t('noAccess.title')} description={t('noAccess.description')} /> : <Outlet />}
  </Page>
}

export function ReportsIndex() {
  const { t } = useTranslation('reports')
  const { hasPermission } = useSession()
  return <nav aria-label={t('library.title')} className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
    {allowedReportTabs(hasPermission).map((item) => <Link key={item.path} to={item.path} className="grid gap-4 rounded-xl border bg-card p-5 hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
      <BarChart3 className="size-6 text-primary" aria-hidden />
      <div className="flex items-center justify-between gap-3"><h2 className="text-base font-semibold">{t(item.labelKey)}</h2><ArrowUpRight className="size-4" aria-hidden /></div>
      <p className="text-sm text-muted-foreground">{t(`library.${item.path}`)}</p>
    </Link>)}
  </nav>
}
